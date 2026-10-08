use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 卫星多普勒频移：输入频率与相对径向速度，估算最大多普勒频移。
#[component]
pub(super) fn DopplerCalculator() -> impl IntoView {
  let freq = RwSignal::new(437.8);
  let vel = RwSignal::new(7.5);

  let freq_id = unique_id("doppler-freq");
  let vel_id = unique_id("doppler-vel");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.downlink-frequency-mhz")) r#for=freq_id.clone()>
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
      <Field label=Signal::derive(move || t("tools.relative-radial-velocity-km")) r#for=vel_id.clone()>
        <NumberField
          id=vel_id
          value=Signal::derive(move || vel.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              vel.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let f = freq.get();
          let v = vel.get();
          if f <= 0.0 || v < 0.0 {
            t("tools.enter-a-positive-frequency-2")
          } else {
            // Δf = f · v / c，c = 299792.458 km/s。
            let shift_khz = f * v / 299_792.458 * 1000.0;
            tf(
              "tools.max-doppler-shift-khz",
              &[&fmt_num(shift_khz), &fmt_num(f)],
            )
          }
        }}
      </div>
    </div>
  }
}
