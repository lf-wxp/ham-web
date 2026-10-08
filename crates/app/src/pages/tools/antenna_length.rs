use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 天线长度估算：半波偶极 143/f、1/4 波长 71.5/f、5/8 波长 187.5/f（米，f 为 MHz）。
#[component]
pub(super) fn AntennaLength() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let freq_id = unique_id("antenna-length-freq");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
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
      <div class=RESULT>
        {move || {
          let f = freq.get();
          if f <= 0.0 {
            t("tools.enter-a-positive-frequency")
          } else {
            tf(
              "tools.half-wave-dipole-m",
              &[
                &fmt_num(143.0 / f),
                &fmt_num(71.5 / f),
                &fmt_num(187.5 / f),
              ],
            )
          }
        }}
      </div>
    </div>
  }
}
