use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 卫星多普勒频移：输入频率与相对径向速度，估算最大多普勒频移。
#[component]
pub(super) fn DopplerCalculator() -> impl IntoView {
  let freq = RwSignal::new(437.8);
  let vel = RwSignal::new(7.5);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("下行频率（MHz）")}</span>
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
        <span class="text-xs text-muted-foreground">{move || t("相对径向速度（km/s，LEO 约 7.5）")}</span>
        <input
          type="number"
          prop:value=move || vel.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              vel.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let f = freq.get();
          let v = vel.get();
          if f <= 0.0 || v < 0.0 {
            t("请输入正频率与速度")
          } else {
            // Δf = f · v / c，c = 299792.458 km/s。
            let shift_khz = f * v / 299_792.458 * 1000.0;
            tf(
              "最大多普勒频移 ≈ {} kHz（{} MHz 处）。卫星接近时频率升高、远离时降低。",
              &[&fmt_num(shift_khz), &fmt_num(f)],
            )
          }
        }}
      </div>
    </div>
  }
}
