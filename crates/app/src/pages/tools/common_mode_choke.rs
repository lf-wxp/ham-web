use ham_web_core::common_mode_choke::{MATERIALS, MAX_TURNS, band_rows, material, valid_turns};
use ham_web_core::toroid::{CORES, core_by_part};
use leptos::prelude::*;

use super::toroid_winding::part_size;
use super::{fmt_num, result_class};
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, SelectOption, SimpleSelect, Table, TableBody, TableCell, TableDensity,
  TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 默认磁环：业余圈里馈线共模扼流最常用的 FT-240-43。
const DEFAULT_PART: &str = "5943003801";

/// 共模扼流圈：选磁环与匝数，按厂商公布的复磁导率估算 160–6 m 各波段的串联阻抗与插入损耗。
#[component]
pub(super) fn CommonModeChoke() -> impl IntoView {
  let part = RwSignal::new(
    core_by_part(DEFAULT_PART)
      .or(CORES.first())
      .map_or_else(String::new, |c| c.part.to_owned()),
  );
  let turns = RwSignal::new(10.0_f64);
  let loop_ohm = RwSignal::new(50.0_f64);

  let part_id = unique_id("cmc-core");
  let turns_id = unique_id("cmc-turns");
  let loop_id = unique_id("cmc-loop");

  // 没有复磁导率表的材料算不出结果：不进下拉，免得选了只看到一句「不合法」。
  let options: Vec<SelectOption> = CORES
    .iter()
    .filter(|c| material(c.material).is_some())
    .map(|c| {
      SelectOption::new(
        c.part.to_owned(),
        format!("{} · {} — {}", c.part, c.material, part_size(c)),
      )
    })
    .collect();
  let material_ids = MATERIALS
    .iter()
    .map(|m| m.id)
    .collect::<Vec<_>>()
    .join(" / ");
  let max_turns = MAX_TURNS.to_string();

  // 匝数 / 回路阻抗不合法时整张表都算不出：给一句话，而不是一张全是「—」的表。
  let report = Memo::new(move |_| {
    let c = core_by_part(&part.get())?;
    let (n, r) = (turns.get(), loop_ohm.get());
    if !(valid_turns(n) && r > 0.0 && r.is_finite()) {
      return None;
    }
    Some((*c, band_rows(c, n, r)))
  });

  view! {
    // 同 `ToroidWinding`：下拉里的料号 + 尺寸是整行不换行的文字，网格子项不加 `min-w-0` 会把整列撑宽，
    // 手机宽度下整页横向滚动。
    <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
      <Field
        class="sm:col-span-2"
        label=Signal::derive(move || t("tools.cmc-core"))
        hint=Signal::derive(move || tf("tools.cmc-core-hint", &[&material_ids]))
        r#for=part_id.clone()
      >
        <SimpleSelect
          id=part_id
          value=part
          on_change=Callback::new(move |v: String| part.set(v))
          options=options
          aria_label=Signal::derive(move || t("tools.cmc-core"))
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.cmc-turns"))
        hint=Signal::derive(move || t("tools.cmc-turns-hint"))
        r#for=turns_id.clone()
      >
        <NumberField
          id=turns_id
          value=Signal::derive(move || turns.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              turns.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.cmc-loop"))
        hint=Signal::derive(move || t("tools.cmc-loop-hint"))
        r#for=loop_id.clone()
      >
        <NumberField
          id=loop_id
          value=Signal::derive(move || loop_ohm.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              loop_ohm.set(v);
            }
          })
          controls=false
        />
      </Field>

      <div class=format!("{} sm:col-span-2 space-y-1 tabular-nums", result_class()) role="status">
        {move || match report.get() {
          None => view! { <span>{tf("tools.cmc-enter-valid", &[&max_turns])}</span> }.into_any(),
          Some((c, _)) => {
            let fetched = material(c.material).map_or("", |m| m.fetched);
            view! {
              <div>
                {tf(
                  "tools.cmc-core-spec",
                  &[c.part, c.material, &fmt_num(c.ae_cm2), &fmt_num(c.le_cm)],
                )}
              </div>
              <div>{tf("tools.cmc-source", &[c.material, fetched])}</div>
            }
            .into_any()
          }
        }}
      </div>

      {move || {
        report.get().map(|(_, rows)| {
          view! {
            <div class="sm:col-span-2">
              <Table aria_label=Signal::derive(move || t("tools.cmc-table")) density=TableDensity::Compact>
                <TableHead class="text-xs text-muted-foreground">
                  <TableRow>
                    <TableHeader scope="col" class="px-2 py-1.5 font-normal">{move || t("radio.band")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.frequency-mhz")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.cmc-col-z")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.cmc-col-r")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.cmc-col-x")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.cmc-col-share")}</TableHeader>
                    <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("tools.cmc-col-loss")}</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody class="font-mono tabular-nums">
                  {rows
                    .into_iter()
                    .map(|r| {
                      let dash = || "—".to_owned();
                      let (mag, res, rea, share) = match r.z {
                        Some(z) => (
                          fmt_num(z.magnitude()),
                          fmt_num(z.r_ohm),
                          fmt_num(z.x_ohm),
                          format!("{:.0}", z.resistive_share() * 100.0),
                        ),
                        None => (dash(), dash(), dash(), dash()),
                      };
                      let loss = r.loss_db.map_or_else(dash, fmt_num);
                      view! {
                        <TableRow>
                          <TableCell class="px-2 py-1">{r.band}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{fmt_num(r.f_mhz)}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{mag}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{res}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{rea}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{share}</TableCell>
                          <TableCell class="px-2 py-1 text-right">{loss}</TableCell>
                        </TableRow>
                      }
                    })
                    .collect_view()}
                </TableBody>
              </Table>
            </div>
          }
        })
      }}
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.cmc-note")}</p>
    </div>
  }
}
