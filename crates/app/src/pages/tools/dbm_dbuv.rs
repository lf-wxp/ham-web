use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// dBm ↔ dBμV 换算（50Ω 阻抗）。
#[component]
pub(super) fn DbmDbuv() -> impl IntoView {
  let dbm = RwSignal::new(0.0);
  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"功率电平（dBm）"</span>
        <input
          type="number"
          prop:value=move || dbm.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              dbm.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || format!("{} dBμV（50Ω）", fmt_num(dbm.get() + 107.0))}
      </div>
    </div>
  }
}
