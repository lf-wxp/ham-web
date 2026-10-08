use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 线圈电感与 Yagi 振子计算。
#[component]
pub(super) fn CoilYagiCalculator() -> impl IntoView {
  let coil_d = RwSignal::new(2.0);
  let coil_l = RwSignal::new(2.0);
  let coil_n = RwSignal::new(10.0);
  let freq = RwSignal::new(14.2);

  let coil_d_id = unique_id("coil-yagi-d");
  let coil_l_id = unique_id("coil-yagi-l");
  let coil_n_id = unique_id("coil-yagi-n");
  let freq_id = unique_id("coil-yagi-freq");

  view! {
    <div class="space-y-4">
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">{move || t("tools.air-core-coil-inductance-2")}</div>
        <div class="grid gap-3 sm:grid-cols-3">
          <Field label=Signal::derive(move || t("tools.diameter-d-cm")) r#for=coil_d_id.clone()>
            <NumberField
              id=coil_d_id
              step=0.1
              value=Signal::derive(move || coil_d.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(v) = v.trim().parse::<f64>() {
                  coil_d.set(v);
                }
              })
              controls=false
            />
          </Field>
          <Field label=Signal::derive(move || t("tools.length-l-cm")) r#for=coil_l_id.clone()>
            <NumberField
              id=coil_l_id
              step=0.1
              value=Signal::derive(move || coil_l.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(v) = v.trim().parse::<f64>() {
                  coil_l.set(v);
                }
              })
              controls=false
            />
          </Field>
          <Field label=Signal::derive(move || t("tools.turns-n")) r#for=coil_n_id.clone()>
            <NumberField
              id=coil_n_id
              step=1.0
              value=Signal::derive(move || coil_n.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(v) = v.trim().parse::<f64>() {
                  coil_n.set(v);
                }
              })
              controls=false
            />
          </Field>
        </div>
        <div class="mt-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
          {move || {
            let d = coil_d.get();
            let l = coil_l.get();
            let n = coil_n.get();
            if d <= 0.0 || n <= 0.0 {
              t("tools.enter-a-positive-diameter")
            } else {
              let denom = 45.4 * d + 100.0 * l;
              let ind = if denom > 0.0 { d * d * n * n / denom } else { 0.0 };
              tf("tools.inductance-h", &[&(fmt_num(ind)).to_string()])
            }
          }}
        </div>
      </div>
      <div>
        <div class="mb-1.5 text-xs font-medium text-muted-foreground">{move || t("tools.yagi-element-lengths-3")}</div>
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
        </div>
        <div class="mt-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
          {move || {
            let f = freq.get();
            if f <= 0.0 {
              t("tools.enter-a-positive-frequency")
            } else {
              tf(
                "tools.reflector-m-driven-m",
                &[
                  &fmt_num(150.0 / f),
                  &fmt_num(143.0 / f),
                  &fmt_num(136.0 / f),
                ],
              )
            }
          }}
        </div>
      </div>
    </div>
  }
}
