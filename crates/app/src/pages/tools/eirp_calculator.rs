use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// EIRP 有效辐射功率：功率 + 天线增益 − 馈线损耗。
#[component]
pub(super) fn EirpCalculator() -> impl IntoView {
  let power = RwSignal::new(50.0);
  let gain = RwSignal::new(3.0);
  let loss = RwSignal::new(1.0);

  let power_id = unique_id("eirp-power");
  let gain_id = unique_id("eirp-gain");
  let loss_id = unique_id("eirp-loss");

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
      <Field label=Signal::derive(move || t("tools.transmit-power-w")) r#for=power_id.clone()>
        <NumberField
          id=power_id
          value=Signal::derive(move || power.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              power.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.antenna-gain-dbi")) r#for=gain_id.clone()>
        <NumberField
          id=gain_id
          value=Signal::derive(move || gain.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              gain.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.feedline-loss-db")) r#for=loss_id.clone()>
        <NumberField
          id=loss_id
          value=Signal::derive(move || loss.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              loss.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let p = power.get();
          if p <= 0.0 {
            t("tools.enter-a-positive-power")
          } else {
            let p_dbm = 10.0 * p.log10() + 30.0;
            let eirp = p_dbm + gain.get() - loss.get();
            format!(
              "EIRP ≈ {} dBm（{} dBm + {} dBi − {} dB）",
              fmt_num(eirp),
              fmt_num(p_dbm),
              fmt_num(gain.get()),
              fmt_num(loss.get()),
            )
          }
        }}
      </div>
    </div>
  }
}
