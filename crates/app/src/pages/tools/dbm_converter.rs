use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// dBm ↔ 功率换算。
#[component]
pub(super) fn DbmConverter() -> impl IntoView {
  let dbm = RwSignal::new(30.0);
  let watts = RwSignal::new(1.0);

  let on_dbm = move |e: web_sys::Event| {
    if let Ok(d) = event_target_value(&e).parse::<f64>() {
      dbm.set(d);
      watts.set(10f64.powf((d - 30.0) / 10.0));
    }
  };
  let on_watt = move |e: web_sys::Event| {
    if let Ok(w) = event_target_value(&e).parse::<f64>()
      && w > 0.0
    {
      watts.set(w);
      dbm.set(30.0 + 10.0 * w.log10());
    }
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"功率电平（dBm）"</span>
        <input type="number" prop:value=move || dbm.get().to_string() on:input=on_dbm class=INPUT />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"功率（W）"</span>
        <input type="number" prop:value=move || watts.get().to_string() on:input=on_watt class=INPUT />
      </label>
      <div class=RESULT>
        {move || {
          let w = watts.get();
          format!(
            "{} dBm = {} W = {} mW",
            fmt_num(dbm.get()),
            fmt_num(w),
            fmt_num(w * 1000.0),
          )
        }}
      </div>
    </div>
  }
}
