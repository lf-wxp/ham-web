//! 磁环小环天线：由环径、导体直径、频率估算电感 / 效率 / Q / 调谐电容 / 电容电压。

use ham_web_core::magnetic_loop::{self, LoopParams};
use ham_web_core::unit_math::{format_capacitance_f, format_inductance_h, format_voltage_v};
use leptos::prelude::*;

use super::{RESULT, fmt_resistance};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

#[component]
pub(super) fn MagneticLoopTool() -> impl IntoView {
  let diameter = RwSignal::new(String::from("1"));
  let conductor = RwSignal::new(String::from("10"));
  let freq = RwSignal::new(String::from("14.2"));
  let turns = RwSignal::new(String::from("1"));
  let power = RwSignal::new(String::from("100"));

  let diameter_id = unique_id("magloop-diameter");
  let conductor_id = unique_id("magloop-conductor");
  let freq_id = unique_id("magloop-freq");
  let turns_id = unique_id("magloop-turns");
  let power_id = unique_id("magloop-power");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.magloop-diameter")) r#for=diameter_id.clone()>
        <NumberField id=diameter_id value=Signal::derive(move || diameter.get()) on_change=Callback::new(move |v: String| diameter.set(v)) step=0.1 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.magloop-conductor")) r#for=conductor_id.clone()>
        <NumberField id=conductor_id value=Signal::derive(move || conductor.get()) on_change=Callback::new(move |v: String| conductor.set(v)) step=0.1 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField id=freq_id value=Signal::derive(move || freq.get()) on_change=Callback::new(move |v: String| freq.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.magloop-turns")) r#for=turns_id.clone()>
        <NumberField id=turns_id value=Signal::derive(move || turns.get()) on_change=Callback::new(move |v: String| turns.set(v)) step=1.0 min=1.0 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.transmit-power-w")) r#for=power_id.clone()>
        <NumberField id=power_id value=Signal::derive(move || power.get()) on_change=Callback::new(move |v: String| power.set(v)) controls=false />
      </Field>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || {
          let result = match (
            diameter.get().trim().parse::<f64>(),
            conductor.get().trim().parse::<f64>(),
            freq.get().trim().parse::<f64>(),
            turns.get().trim().parse::<f64>(),
            power.get().trim().parse::<f64>(),
          ) {
            (Ok(dm), Ok(cm), Ok(fm), Ok(n), Ok(pw)) => magnetic_loop::magnetic_loop(LoopParams {
              diameter_m: dm,
              conductor_mm: cm,
              freq_mhz: fm,
              turns: n,
              power_w: pw,
            }),
            _ => None,
          };
          match result {
            None => view! { <p>{move || t("tools.magloop-invalid")}</p> }.into_any(),
            Some(l) => view! {
              <p>{tf("tools.magloop-l-c", &[&format_inductance_h(l.inductance_h), &format_capacitance_f(l.tuning_capacitance_f)])}</p>
              <p>{tf("tools.magloop-r-eff", &[&fmt_resistance(l.radiation_resistance), &fmt_resistance(l.loss_resistance), &format!("{:.0}%", l.efficiency * 100.0)])}</p>
              <p>{tf("tools.magloop-q-bw", &[&format!("{:.0}", l.q), &format!("{:.1} kHz", l.bandwidth_hz / 1e3)])}</p>
              <p>{tf("tools.magloop-voltage", &[&format_voltage_v(l.cap_voltage)])}</p>
              // 小环近似有前提（周长 ≤ λ/2π）：出了适用域要在结果下面直说，别让读数被当成准数。
              {(!l.electrically_small).then(|| view! {
                <p class="text-amber-600 dark:text-amber-400">{move || t("tools.magloop-out-of-range")}</p>
              })}
            }
              .into_any(),
          }
        }}
      </div>
    </div>
  }
}
