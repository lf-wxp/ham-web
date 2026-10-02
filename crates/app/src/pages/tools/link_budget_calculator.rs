use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 链路预算：到达功率 = EIRP − 路径损耗 + 接收增益。
#[component]
pub(super) fn LinkBudgetCalculator() -> impl IntoView {
  let power = RwSignal::new(50.0);
  let tx_gain = RwSignal::new(3.0);
  let dist = RwSignal::new(500.0);
  let freq = RwSignal::new(145.0);
  let rx_gain = RwSignal::new(3.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("发射功率（W）")}</span>
        <input
          type="number"
          prop:value=move || power.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              power.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("发射天线增益（dBi）")}</span>
        <input
          type="number"
          prop:value=move || tx_gain.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              tx_gain.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("距离（km）")}</span>
        <input
          type="number"
          prop:value=move || dist.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              dist.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("频率（MHz）")}</span>
        <input
          type="number"
          prop:value=move || freq.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              freq.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("接收天线增益（dBi）")}</span>
        <input
          type="number"
          prop:value=move || rx_gain.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              rx_gain.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let (p, d, f) = (power.get(), dist.get(), freq.get());
          if p <= 0.0 || d <= 0.0 || f <= 0.0 {
            t("请输入正的功率、距离与频率")
          } else {
            let eirp = 10.0 * p.log10() + 30.0 + tx_gain.get();
            let fspl = 20.0 * d.log10() + 20.0 * f.log10() + 32.45;
            let rx = eirp - fspl + rx_gain.get();
            tf(
              "到达功率 ≈ {} dBm（EIRP {} − 路径损耗 {} + 接收增益 {}）",
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
