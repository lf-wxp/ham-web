use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};

/// CW 必要带宽估算：Bn = B × K，B = WPM / 1.2，K 取 5（衰落信道）或 3（非衰落）。
#[component]
pub(super) fn CwBandwidth() -> impl IntoView {
  let wpm = RwSignal::new(25.0);
  let fading = RwSignal::new(true);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"拍发速度（WPM）"</span>
        <input
          type="number"
          prop:value=move || wpm.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              wpm.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="flex items-end gap-1.5 pb-1">
        <button
          type="button"
          on:click=move |_| fading.set(true)
          class=move || {
            if fading.get() {
              "inline-flex items-center whitespace-nowrap rounded-full border bg-primary px-3 py-1 text-xs text-primary-foreground"
            } else {
              "inline-flex items-center whitespace-nowrap rounded-full border px-3 py-1 text-xs hover:bg-accent"
            }
          }
        >
          "衰落 K=5"
        </button>
        <button
          type="button"
          on:click=move |_| fading.set(false)
          class=move || {
            if !fading.get() {
              "inline-flex items-center whitespace-nowrap rounded-full border bg-primary px-3 py-1 text-xs text-primary-foreground"
            } else {
              "inline-flex items-center whitespace-nowrap rounded-full border px-3 py-1 text-xs hover:bg-accent"
            }
          }
        >
          "非衰落 K=3"
        </button>
      </div>
      <div class=RESULT>
        {move || {
          let k = if fading.get() { 5.0 } else { 3.0 };
          let b = wpm.get() / 1.2;
          format!("波特率 B = {} Bd，必要带宽 Bn = {} Hz", fmt_num(b), fmt_num(b * k))
        }}
      </div>
    </div>
  }
}
