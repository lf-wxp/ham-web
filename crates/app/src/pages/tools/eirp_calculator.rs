use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::t;

/// EIRP 有效辐射功率：功率 + 天线增益 − 馈线损耗。
#[component]
pub(super) fn EirpCalculator() -> impl IntoView {
  let power = RwSignal::new(50.0);
  let gain = RwSignal::new(3.0);
  let loss = RwSignal::new(1.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-3">
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
        <span class="text-xs text-muted-foreground">{move || t("天线增益（dBi）")}</span>
        <input
          type="number"
          prop:value=move || gain.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              gain.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("馈线损耗（dB）")}</span>
        <input
          type="number"
          prop:value=move || loss.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              loss.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-3 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let p = power.get();
          if p <= 0.0 {
            t("请输入正功率")
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
