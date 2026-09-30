use leptos::prelude::*;

use super::INPUT;

/// 馈线损耗估算。
#[component]
pub(super) fn FeedlineLoss() -> impl IntoView {
  let loss_per_100m = RwSignal::new(2.0);
  let length = RwSignal::new(30.0);
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"馈线损耗（dB / 100m）"</span>
        <input
          type="number"
          step="0.1"
          prop:value=move || loss_per_100m.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              loss_per_100m.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"馈线长度（m）"</span>
        <input
          type="number"
          step="1"
          prop:value=move || length.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              length.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let db = loss_per_100m.get() * length.get() / 100.0;
          let power_pct = (1.0 - 10f64.powf(-db / 10.0)) * 100.0;
          format!("总损耗 {:.2} dB　功率损耗 {:.1}%", db, power_pct)
        }}
      </div>
    </div>
  }
}
