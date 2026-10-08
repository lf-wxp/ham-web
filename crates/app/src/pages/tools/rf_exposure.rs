use ham_web_core::rf_exposure::assess;
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 射频暴露合规评估：按 FCC OET-65 估算功率密度与最小安全距离。
#[component]
pub(super) fn RfExposure() -> impl IntoView {
  let power = RwSignal::new(100.0);
  let freq = RwSignal::new(14.0);
  let gain = RwSignal::new(2.15);
  let distance = RwSignal::new(10.0);

  let power_id = unique_id("rf-exposure-power");
  let freq_id = unique_id("rf-exposure-freq");
  let gain_id = unique_id("rf-exposure-gain");
  let distance_id = unique_id("rf-exposure-distance");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
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
      <Field label=Signal::derive(move || t("tools.evaluation-distance-m")) r#for=distance_id.clone()>
        <NumberField
          id=distance_id
          value=Signal::derive(move || distance.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              distance.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        {move || {
          match assess(power.get(), freq.get(), gain.get(), distance.get()) {
            Some(r) => {
              let ok_public = r.power_density <= r.mpe_uncontrolled;
              view! {
                <div class="tabular-nums">
                  {tf("tools.power-density-mw-cm", &[&fmt_num(r.power_density)])}
                </div>
                <div class="tabular-nums">
                  {tf(
                    "tools.public-limit-mw-cm",
                    &[&fmt_num(r.mpe_uncontrolled), &fmt_num(r.mpe_controlled)],
                  )}
                </div>
                <div class="tabular-nums">
                  {tf(
                    "tools.minimum-safe-distance-m",
                    &[
                      &fmt_num(r.safe_distance_uncontrolled_m),
                      &fmt_num(r.safe_distance_controlled_m),
                    ],
                  )}
                </div>
                <div>
                  {move || {
                    if ok_public {
                      t("tools.within-the-public-limit")
                    } else {
                      t("tools.exceeds-the-public-limit")
                    }
                  }}
                </div>
              }
              .into_any()
            }
            None => view! { <span>{move || t("tools.enter-positive-power-frequency")}</span> }.into_any(),
          }
        }}
      </div>
      <p class="sm:col-span-2 text-xs text-muted-foreground">
        {move || t("tools.far-field-approximation-s")}
      </p>
    </div>
  }
}
