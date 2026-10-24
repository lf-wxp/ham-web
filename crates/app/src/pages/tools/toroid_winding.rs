use ham_web_core::toroid::{
  CORES, Core, core_by_part, inductance_uh, length_per_turn_mm, max_turns_single_layer, winding_for,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf, tp};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

/// 下拉里的「自定义」选项值。
const CUSTOM: &str = "custom";

/// 磁环绕线计算：由目标电感反推匝数（或由匝数算电感），并给出导线长度与窗口占用提醒。
#[component]
pub(super) fn ToroidWinding() -> impl IntoView {
  // 默认选第一个已收录的磁环（下拉有选项时 `select` 不会落到「自定义」）。
  let part = RwSignal::new(
    CORES
      .first()
      .map_or(CUSTOM.to_owned(), |c| c.part.to_owned()),
  );
  let custom_al = RwSignal::new(1000.0);
  let custom_tol = RwSignal::new(20.0);
  let target = RwSignal::new(50.0);
  let wire_od = RwSignal::new(0.6);

  let part_id = unique_id("toroid-part");
  let al_id = unique_id("toroid-al");
  let tol_id = unique_id("toroid-tol");
  let target_id = unique_id("toroid-target");
  let wire_id = unique_id("toroid-wire");

  let options: Vec<SelectOption> = CORES
    .iter()
    .map(|c| {
      SelectOption::new(
        c.part.to_owned(),
        format!("{} · {} — {}", c.part, c.material, part_size(c)),
      )
    })
    .chain(std::iter::once(SelectOption::new(
      CUSTOM,
      Signal::derive(move || t("tools.toroid-custom")),
    )))
    .collect();

  // `Core` 的字段是 `Copy` 的，但整体按值取用更简单：自定义时现造一个。
  let core = Memo::new(move |_| -> Core {
    match core_by_part(&part.get()) {
      Some(c) => *c,
      None => Core {
        part: "custom",
        material: "",
        od_mm: 0.0,
        id_mm: 0.0,
        height_mm: 0.0,
        al_nh: custom_al.get(),
        tolerance_pct: custom_tol.get(),
        mu_i: 0.0,
        le_cm: 0.0,
        ae_cm2: 0.0,
        verified: "",
      },
    }
  });

  let report = Memo::new(move |_| {
    let c = core.get();
    // 自定义磁环没有尺寸：导线长度与窗口占用都算不了，但匝数与电感照常给。
    let sized = c.id_mm > 0.0;
    let per_turn = sized.then(|| length_per_turn_mm(&c));
    let max_turns = sized
      .then(|| max_turns_single_layer(&c, wire_od.get()))
      .flatten();
    let winding = winding_for(c.al_nh, c.tolerance_pct, target.get(), per_turn, max_turns)?;
    Some((c, winding, max_turns))
  });

  view! {
    // `[&>*]:min-w-0`：网格子项默认 `min-width:auto`，不收缩；下拉里的料号 + 尺寸是一整行不换行的文字，
    // 不加这个会把整列撑到 400px 以上，手机宽度下整页横向滚动（触发器内部的 `truncate` 也就没机会生效）。
    <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
      <Field
        class="sm:col-span-2"
        label=Signal::derive(move || t("tools.toroid-core"))
        hint=Signal::derive(move || t("tools.toroid-core-hint"))
        r#for=part_id.clone()
      >
        <NativeSelect
          id=part_id
          value=part
          on_change=Callback::new(move |v: String| part.set(v))
          options=options
          aria_label=Signal::derive(move || t("tools.toroid-core"))
        />
      </Field>

      {move || {
        // 每次渲染都先克隆：`Field` 的子内容是 `move` 闭包，直接用会把 id 移出外层闭包，
        // 外层就只能执行一次，条件切换时无法重新渲染。
        core_by_part(&part.get()).is_none().then(|| {
          let (al_id, tol_id) = (al_id.clone(), tol_id.clone());
          view! {
        <Field label=Signal::derive(move || t("tools.toroid-al")) r#for=al_id.clone()>
          <NumberField
            id=al_id
            value=Signal::derive(move || custom_al.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                custom_al.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.toroid-al-tolerance")) r#for=tol_id.clone()>
          <NumberField
            id=tol_id
            value=Signal::derive(move || custom_tol.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                custom_tol.set(v);
              }
            })
            controls=false
          />
        </Field>
      }
        })
      }}

      <Field label=Signal::derive(move || t("tools.toroid-target-uh")) r#for=target_id.clone()>
        <NumberField
          id=target_id
          value=Signal::derive(move || target.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              target.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.toroid-wire-od")) r#for=wire_id.clone()>
        <NumberField
          id=wire_id
          value=Signal::derive(move || wire_od.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              wire_od.set(v);
            }
          })
          controls=false
        />
      </Field>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || match report.get() {
          None => view! { <span>{move || t("tools.toroid-enter-valid")}</span> }.into_any(),
          Some((c, w, max_turns)) => {
            let spec = if c.part == "custom" {
              tf("tools.toroid-custom-spec", &[&fmt_num(c.al_nh), &fmt_num(c.tolerance_pct)])
            } else {
              tf(
                "tools.toroid-core-spec",
                &[
                  c.part,
                  c.material,
                  &fmt_num(c.od_mm),
                  &fmt_num(c.id_mm),
                  &fmt_num(c.height_mm),
                  &fmt_num(c.al_nh),
                  &fmt_num(c.tolerance_pct),
                  &fmt_num(c.mu_i),
                ],
              )
            };
            let turns = tp(
              "tools.toroid-turns-result",
              w.turns,
              &[&w.turns.to_string(), &fmt_num(w.turns_exact)],
            );
            let actual = tf(
              "tools.toroid-actual-result",
              &[&fmt_num(w.actual_uh), &fmt_num(w.low_uh), &fmt_num(w.high_uh)],
            );
            let wire = w
              .wire_m
              .map(|m| tf("tools.toroid-wire-result", &[&fmt_num(m)]));
            let window = match (max_turns, w.exceeds_single_layer) {
              (Some(max), Some(_)) => {
                tp("tools.toroid-window-over", max, &[&max.to_string()])
              }
              (Some(max), None) => tp("tools.toroid-window-ok", max, &[&max.to_string()]),
              _ => t("tools.toroid-window-unknown"),
            };
            let one_turn = fmt_num(inductance_uh(c.al_nh, 1.0).unwrap_or_default());
            view! {
              <div>{spec}</div>
              <div role="status" class="text-foreground">{turns}</div>
              <div>{actual}</div>
              {wire.map(|text| view! { <div>{text}</div> })}
              <div>{window}</div>
              <div>{tf("tools.toroid-one-turn", &[&one_turn])}</div>
            }
            .into_any()
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.toroid-note")}</p>
    </div>
  }
}

/// 「61 × 35.55 × 12.7 mm」；自定义磁环没有尺寸。
fn part_size(c: &Core) -> String {
  format!("{} × {} × {} mm", c.od_mm, c.id_mm, c.height_mm)
}
