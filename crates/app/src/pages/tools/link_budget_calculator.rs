use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 链路预算：到达功率 = EIRP − 路径损耗 + 接收增益。
#[component]
pub(super) fn LinkBudgetCalculator() -> impl IntoView {
  let power = RwSignal::new(50.0);
  let tx_gain = RwSignal::new(3.0);
  let dist = RwSignal::new(500.0);
  let freq = RwSignal::new(145.0);
  let rx_gain = RwSignal::new(3.0);

  let power_id = unique_id("link-budget-power");
  let tx_gain_id = unique_id("link-budget-tx-gain");
  let dist_id = unique_id("link-budget-dist");
  let freq_id = unique_id("link-budget-freq");
  let rx_gain_id = unique_id("link-budget-rx-gain");

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
      <Field label=Signal::derive(move || t("tools.transmit-antenna-gain-dbi")) r#for=tx_gain_id.clone()>
        <NumberField
          id=tx_gain_id
          value=Signal::derive(move || tx_gain.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              tx_gain.set(v);
            }
          })
          controls=false
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField
          id=dist_id
          value=Signal::derive(move || dist.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              dist.set(v);
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
      <Field label=Signal::derive(move || t("tools.receive-antenna-gain-dbi")) r#for=rx_gain_id.clone()>
        <NumberField
          id=rx_gain_id
          value=Signal::derive(move || rx_gain.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              rx_gain.set(v);
            }
          })
          controls=false
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let (p, d, f) = (power.get(), dist.get(), freq.get());
          if p <= 0.0 || d <= 0.0 || f <= 0.0 {
            t("tools.enter-a-positive-power-2")
          } else {
            let eirp = 10.0 * p.log10() + 30.0 + tx_gain.get();
            let fspl = 20.0 * d.log10() + 20.0 * f.log10() + 32.45;
            let rx = eirp - fspl + rx_gain.get();
            tf(
              "tools.received-power-dbm-eirp",
              &[
                &fmt_num(rx),
                &fmt_num(eirp),
                &fmt_num(fspl),
                &fmt_num(rx_gain.get()),
              ],
            )
          }
        }}
      </div>
    </div>
  }
}
