//! 电短天线加载线圈：短单极子（车载鞭天线）谐振所需的加载电感。

use ham_web_core::loading_coil::{self, LoadingCoilInput};
use ham_web_core::unit_math::{format_capacitance_f, format_inductance_h};
use leptos::prelude::*;

use super::{RESULT, fmt_resistance};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

#[component]
pub(super) fn LoadingCoilTool() -> impl IntoView {
  let height = RwSignal::new(String::from("2.4"));
  let conductor = RwSignal::new(String::from("2"));
  let freq = RwSignal::new(String::from("7"));

  let height_id = unique_id("loading-coil-height");
  let conductor_id = unique_id("loading-coil-conductor");
  let freq_id = unique_id("loading-coil-freq");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.loading-coil-height")) r#for=height_id.clone()>
        <NumberField id=height_id.clone() value=Signal::derive(move || height.get()) on_change=Callback::new(move |v: String| height.set(v)) step=0.1 controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.loading-coil-conductor")) r#for=conductor_id.clone()>
        <NumberField id=conductor_id.clone() value=Signal::derive(move || conductor.get()) on_change=Callback::new(move |v: String| conductor.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField id=freq_id.clone() value=Signal::derive(move || freq.get()) on_change=Callback::new(move |v: String| freq.set(v)) controls=false />
      </Field>

      <div class=format!("{RESULT} space-y-1 tabular-nums")>
        {move || {
          let (Ok(h), Ok(d), Ok(f)) = (
            height.get().trim().parse::<f64>(),
            conductor.get().trim().parse::<f64>(),
            freq.get().trim().parse::<f64>(),
          ) else {
            return view! { <p>{move || t("tools.loading-coil-invalid")}</p> }.into_any();
          };
          match loading_coil::loading_coil(LoadingCoilInput { height_m: h, conductor_mm: d, freq_mhz: f }) {
            None => view! { <p>{move || t("tools.loading-coil-invalid")}</p> }.into_any(),
            Some(r) => view! {
              <p>{tf("tools.loading-coil-rr-c", &[&fmt_resistance(r.radiation_resistance), &format_capacitance_f(r.capacitance_f)])}</p>
              <p>{tf("tools.loading-coil-xc-l", &[&fmt_resistance(r.capacitive_reactance), &format_inductance_h(r.inductance_h)])}</p>
              // h 超过 λ/4 后闭式近似迅速失真：算出来的值仍要显示，但必须标明只能当量级参考。
              {(!r.electrically_short).then(|| view! {
                <p class="text-amber-600 dark:text-amber-400">{move || t("tools.loading-coil-out-of-range")}</p>
              })}
            }
              .into_any(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.loading-coil-note")}</p>
    </div>
  }
}
