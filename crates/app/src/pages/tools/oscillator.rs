use ham_web_core::oscillator::{crystal_pull_ppm, series_resonance_hz};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 晶体振荡器：频率牵引与串联谐振。
#[component]
pub(super) fn Oscillator() -> impl IntoView {
  let c1 = RwSignal::new(20.0);
  let c0 = RwSignal::new(5.0);
  let cl = RwSignal::new(20.0);
  let l = RwSignal::new(1.0);
  let c = RwSignal::new(1000.0);

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-3">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("动态电容 C1（fF）")}</span>
          <input
            type="number"
            prop:value=move || c1.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                c1.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("静电容 C0（pF）")}</span>
          <input
            type="number"
            prop:value=move || c0.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                c0.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("负载电容 CL（pF）")}</span>
          <input
            type="number"
            prop:value=move || cl.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                cl.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let ppm = crystal_pull_ppm(c1.get() * 1e-15, c0.get() * 1e-12, cl.get() * 1e-12);
          if ppm.is_nan() {
            t("牵引量：请输入正的 C1 与 C0。")
          } else {
            tf("频率牵引 Δf/f ≈ {} ppm", &[&fmt_num(ppm)])
          }
        }}
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("电感（μH）")}</span>
          <input
            type="number"
            prop:value=move || l.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                l.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("电容（pF）")}</span>
          <input
            type="number"
            prop:value=move || c.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                c.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          let f = series_resonance_hz(l.get() * 1e-6, c.get() * 1e-12);
          if f.is_nan() {
            t("串联谐振：请输入正的 L 与 C。")
          } else {
            tf("串联谐振频率 f ≈ {} MHz", &[&fmt_num(f / 1e6)])
          }
        }}
      </div>
    </div>
  }
}
