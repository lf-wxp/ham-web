use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// 频率 ↔ 波长换算（λ = 300 / f(MHz)）。
#[component]
pub(super) fn FreqWavelength() -> impl IntoView {
  let freq = RwSignal::new(145.0);
  let wl = RwSignal::new(300.0 / 145.0);

  let on_freq = move |e: web_sys::Event| {
    if let Ok(f) = event_target_value(&e).parse::<f64>()
      && f > 0.0
    {
      freq.set(f);
      wl.set(300.0 / f);
    }
  };
  let on_wl = move |e: web_sys::Event| {
    if let Ok(w) = event_target_value(&e).parse::<f64>()
      && w > 0.0
    {
      wl.set(w);
      freq.set(300.0 / w);
    }
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
        <input type="number" prop:value=move || freq.get().to_string() on:input=on_freq class=INPUT />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"波长（m）"</span>
        <input type="number" prop:value=move || wl.get().to_string() on:input=on_wl class=INPUT />
      </label>
      <div class=RESULT>
        {move || format!("{} MHz ≈ {} m", fmt_num(freq.get()), fmt_num(wl.get()))}
      </div>
    </div>
  }
}
