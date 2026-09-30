use leptos::prelude::*;

use super::{INPUT, fmt_num};

/// 噪声系数级联（Friis 公式，3 级）。
#[component]
pub(super) fn NoiseCascade() -> impl IntoView {
  let nf1 = RwSignal::new(2.0);
  let g1 = RwSignal::new(15.0);
  let nf2 = RwSignal::new(6.0);
  let g2 = RwSignal::new(20.0);
  let nf3 = RwSignal::new(10.0);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"第 1 级 NF（dB）"</span>
        <input
          type="number"
          prop:value=move || nf1.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              nf1.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"第 1 级增益（dB）"</span>
        <input
          type="number"
          prop:value=move || g1.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              g1.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"第 2 级 NF（dB）"</span>
        <input
          type="number"
          prop:value=move || nf2.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              nf2.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"第 2 级增益（dB）"</span>
        <input
          type="number"
          prop:value=move || g2.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              g2.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">"第 3 级 NF（dB）"</span>
        <input
          type="number"
          prop:value=move || nf3.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              nf3.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let to_lin = |db: f64| 10f64.powf(db / 10.0);
          let f1 = to_lin(nf1.get());
          let f2 = to_lin(nf2.get());
          let f3 = to_lin(nf3.get());
          let g1l = to_lin(g1.get());
          let g2l = to_lin(g2.get());
          let f_total = f1 + (f2 - 1.0) / g1l + (f3 - 1.0) / (g1l * g2l);
          let nf_total = 10.0 * f_total.log10();
          format!("总噪声系数 ≈ {} dB", fmt_num(nf_total))
        }}
      </div>
    </div>
  }
}
