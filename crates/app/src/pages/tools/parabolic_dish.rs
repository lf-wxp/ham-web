use ham_web_core::parabolic_dish::{DEFAULT_EFFICIENCY, DishInput, analyze, focus_from_depth};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 焦点离口径平面不到 1 nm 就当作恰在平面上：浮点运算下 `f/D = 0.25` 的差值不一定是精确的 0。
const ON_RIM_EPS_M: f64 = 1e-9;

/// 抛物面天线：增益、波束宽度、焦点位置，以及用实测深度反推焦距。
#[component]
pub(super) fn ParabolicDish() -> impl IntoView {
  let freq = RwSignal::new(10_368.0);
  let diameter = RwSignal::new(0.9);
  // 百分数：输入框里写 55 比写 0.55 更不容易填错。
  // 取 4 位小数：0.55 × 100 在浮点下是 55.00000000000001，直接放进输入框会原样显示出来。
  let efficiency_pct = RwSignal::new((DEFAULT_EFFICIENCY * 100.0 * 1e4).round() / 1e4);
  let f_over_d = RwSignal::new(0.4);
  let depth = RwSignal::new(0.15);

  let freq_id = unique_id("dish-freq");
  let diameter_id = unique_id("dish-diameter");
  let eta_id = unique_id("dish-efficiency");
  let fd_id = unique_id("dish-f-over-d");
  let depth_id = unique_id("dish-depth");

  let report = Memo::new(move |_| {
    analyze(&DishInput {
      freq_mhz: freq.get(),
      diameter_m: diameter.get(),
      efficiency: efficiency_pct.get() / 100.0,
      f_over_d: f_over_d.get(),
    })
  });
  let measured = Memo::new(move |_| focus_from_depth(diameter.get(), depth.get()));

  view! {
    // `[&>*]:min-w-0`：网格子项默认 `min-width:auto`，长文案会把整列撑宽，手机宽度下整页横向滚动。
    <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField
          id=freq_id
          value=Signal::derive(move || freq.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              freq.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.dish-diameter-m")) r#for=diameter_id.clone()>
        <NumberField
          id=diameter_id
          value=Signal::derive(move || diameter.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              diameter.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.dish-efficiency-pct"))
        hint=Signal::derive(move || t("tools.dish-efficiency-hint"))
        r#for=eta_id.clone()
      >
        <NumberField
          id=eta_id
          value=Signal::derive(move || efficiency_pct.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              efficiency_pct.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.dish-f-over-d")) r#for=fd_id.clone()>
        <NumberField
          id=fd_id
          value=Signal::derive(move || f_over_d.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              f_over_d.set(v);
            }
          })
          controls=false
        />
      </Field>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || match report.get() {
          None => view! { <span>{move || t("tools.dish-enter-valid")}</span> }.into_any(),
          Some(r) => {
            let focus = if r.focus_above_rim_m.abs() < ON_RIM_EPS_M {
              t("tools.dish-focus-on-rim")
            } else if r.focus_above_rim_m > 0.0 {
              tf("tools.dish-focus-outside", &[&fmt_num(r.focus_above_rim_m)])
            } else {
              tf("tools.dish-focus-inside", &[&fmt_num(-r.focus_above_rim_m)])
            };
            view! {
              <div>
                {tf(
                  "tools.dish-wavelength-result",
                  &[&fmt_num(r.wavelength_m), &fmt_num(r.diameter_wavelengths)],
                )}
              </div>
              <div role="status" class="text-foreground">
                {tf(
                  "tools.dish-gain-result",
                  &[
                    &fmt_num(r.gain_dbi),
                    &fmt_num(r.gain_linear),
                    &fmt_num(r.effective_area_m2),
                  ],
                )}
              </div>
              <div>{tf("tools.dish-beamwidth-result", &[&fmt_num(r.beamwidth_deg)])}</div>
              <div>
                {tf(
                  "tools.dish-geometry-result",
                  &[&fmt_num(r.focal_length_m), &fmt_num(r.depth_m)],
                )}
              </div>
              <div>{focus}</div>
              <div>{tf("tools.dish-angle-result", &[&fmt_num(r.subtended_angle_deg)])}</div>
              {r.small_aperture
                .then(|| {
                  view! {
                    <div class="text-hp-text">{move || t("tools.dish-small-aperture")}</div>
                  }
                })}
            }
              .into_any()
          }
        }}
      </div>

      <h3 class="sm:col-span-2 pt-2 text-xs text-foreground">
        {move || t("tools.dish-measure-title")}
      </h3>
      <Field
        label=Signal::derive(move || t("tools.dish-depth-m"))
        hint=Signal::derive(move || t("tools.dish-depth-hint"))
        r#for=depth_id.clone()
      >
        <NumberField
          id=depth_id
          value=Signal::derive(move || depth.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              depth.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class=format!("{RESULT} tabular-nums")>
        {move || match measured.get() {
          None => view! { <span>{move || t("tools.dish-measure-invalid")}</span> }.into_any(),
          Some((f, fd)) => {
            view! {
              <span>
                {tf("tools.dish-measure-result", &[&fmt_num(f), &fmt_num(fd)])}
              </span>
            }
              .into_any()
          }
        }}
      </div>

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.dish-note")}</p>
    </div>
  }
}
