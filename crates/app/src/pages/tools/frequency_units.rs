use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// 频率单位换算：以 MHz 为基准换算到 Hz / kHz / GHz。
#[component]
pub(super) fn FrequencyUnits() -> impl IntoView {
  let mhz = RwSignal::new(145.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
        <input
          type="number"
          prop:value=move || mhz.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              mhz.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let m = mhz.get();
          format!(
            "{} Hz　{} kHz　{} MHz　{} GHz",
            fmt_num(m * 1e6),
            fmt_num(m * 1e3),
            fmt_num(m),
            fmt_num(m / 1e3),
          )
        }}
      </div>
    </div>
  }
}
