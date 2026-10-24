//! 离网供电逐日能量平衡：太阳能板 + 电池 + 负载在最坏月能否自持。

use ham_web_core::off_grid::{self, OffGridInput};
use leptos::prelude::*;

use super::RESULT;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 百分比输入（1–100）→ 0..1 的分数（输入 50 → 0.5）。
///
/// 0 与 >100 都按非法处理：DOD 0% 表示电池一点电都放不出来，效率 0% 表示一点电都充不进去，
/// 都不是有意义的输入；越界由调用方给**专门**的提示，别混进「功率 / 日照无效」那句里。
fn frac(v: &str) -> Option<f64> {
  let p = v.trim().parse::<f64>().ok()?;
  (p > 0.0 && p <= 100.0).then_some(p / 100.0)
}

#[component]
pub(super) fn OffGrid() -> impl IntoView {
  let panel = RwSignal::new(String::from("100"));
  let sun = RwSignal::new(String::from("4"));
  let load = RwSignal::new(String::from("300"));
  let capacity = RwSignal::new(String::from("100"));
  let voltage = RwSignal::new(String::from("12.8"));
  let dod = RwSignal::new(String::from("50"));
  let charge_eff = RwSignal::new(String::from("90"));
  let controller_eff = RwSignal::new(String::from("95"));

  let panel_id = unique_id("offgrid-panel");
  let sun_id = unique_id("offgrid-sun");
  let load_id = unique_id("offgrid-load");
  let capacity_id = unique_id("offgrid-capacity");
  let voltage_id = unique_id("offgrid-voltage");
  let dod_id = unique_id("offgrid-dod");
  let charge_eff_id = unique_id("offgrid-charge-eff");
  let controller_eff_id = unique_id("offgrid-controller-eff");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.offgrid-panel")) r#for=panel_id.clone()>
        <NumberField id=panel_id.clone() value=Signal::derive(move || panel.get()) on_change=Callback::new(move |v: String| panel.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.offgrid-sun"))
        hint=Signal::derive(move || t("tools.offgrid-sun-hint"))
        r#for=sun_id.clone()
      >
        <NumberField id=sun_id.clone() value=Signal::derive(move || sun.get()) on_change=Callback::new(move |v: String| sun.set(v)) step=0.5 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-load")) r#for=load_id.clone()>
        <NumberField id=load_id.clone() value=Signal::derive(move || load.get()) on_change=Callback::new(move |v: String| load.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-capacity")) r#for=capacity_id.clone()>
        <NumberField id=capacity_id.clone() value=Signal::derive(move || capacity.get()) on_change=Callback::new(move |v: String| capacity.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-voltage")) r#for=voltage_id.clone()>
        <NumberField id=voltage_id.clone() value=Signal::derive(move || voltage.get()) on_change=Callback::new(move |v: String| voltage.set(v)) step=0.1 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-dod")) r#for=dod_id.clone()>
        <NumberField id=dod_id.clone() value=Signal::derive(move || dod.get()) on_change=Callback::new(move |v: String| dod.set(v)) step=5.0 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-charge-eff")) r#for=charge_eff_id.clone()>
        <NumberField id=charge_eff_id.clone() value=Signal::derive(move || charge_eff.get()) on_change=Callback::new(move |v: String| charge_eff.set(v)) step=5.0 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.offgrid-controller-eff")) r#for=controller_eff_id.clone()>
        <NumberField id=controller_eff_id.clone() value=Signal::derive(move || controller_eff.get()) on_change=Callback::new(move |v: String| controller_eff.set(v)) step=5.0 controls=false />
      </Field>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || {
          let (Ok(pw), Ok(ps), Ok(ld), Ok(cap), Ok(vv)) = (
            panel.get().trim().parse::<f64>(),
            sun.get().trim().parse::<f64>(),
            load.get().trim().parse::<f64>(),
            capacity.get().trim().parse::<f64>(),
            voltage.get().trim().parse::<f64>(),
          ) else {
            return view! { <p>{move || t("tools.offgrid-invalid")}</p> }.into_any();
          };
          // 三个百分比各有自己的取值范围，超了就直说是哪一类输入不对，不要笼统报「参数无效」。
          let (Some(dod_f), Some(charge_f), Some(controller_f)) =
            (frac(&dod.get()), frac(&charge_eff.get()), frac(&controller_eff.get()))
          else {
            return view! { <p>{move || t("tools.offgrid-invalid-percent")}</p> }.into_any();
          };
          let input = OffGridInput {
            panel_watts: pw,
            peak_sun_hours: ps,
            daily_load_wh: ld,
            battery_ah: cap,
            battery_voltage_v: vv,
            dod: dod_f,
            charge_eff: charge_f,
            controller_eff: controller_f,
          };
          match off_grid::off_grid(input) {
            None => view! { <p>{move || t("tools.offgrid-invalid")}</p> }.into_any(),
            Some(r) => view! {
              <p>{tf("tools.offgrid-summary", &[&format!("{:.0} Wh", r.daily_solar_wh), &format!("{:.0} Wh", r.battery_usable_wh)])}</p>
              {match r.autonomy_days {
                None => view! {
                  <p class="text-win-text">
                    {tf("tools.offgrid-surplus", &[&format!("{:.0} Wh", r.net_wh)])}
                  </p>
                }
                  .into_any(),
                Some(days) => view! {
                  <p class="text-gold-text">
                    {tf("tools.offgrid-deficit", &[&format!("{:.0} Wh", -r.net_wh), &format!("{days:.1}")])}
                  </p>
                }
                  .into_any(),
              }}
            }
              .into_any(),
          }
        }}
      </div>
    </div>
  }
}
