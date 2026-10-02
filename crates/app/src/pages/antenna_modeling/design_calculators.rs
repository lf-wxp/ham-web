use ham_web_core::antenna_design::{vertical_dims, yagi_dims};
use leptos::prelude::*;

use crate::i18n::t;

const INPUT: &str = "h-10 rounded-lg border bg-background px-3 text-sm tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

/// Yagi 振子尺寸计算器。
#[component]
pub(super) fn YagiCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let directors = RwSignal::new(2usize);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{t("频率（MHz）")}</span>
        <input
          type="number"
          step="0.01"
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
        <span class="text-xs text-muted-foreground">{t("引向器数量（0–5）")}</span>
        <input
          type="number"
          min="0"
          max="5"
          step="1"
          prop:value=move || directors.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<usize>() {
              directors.set(v.min(5));
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let Some(d) = yagi_dims(freq.get(), directors.get()) else {
            return t("请输入正频率");
          };
          let mut parts = vec![
            format!("反射器 ≈ {:.2} m", d.reflector),
            format!("激励振子 ≈ {:.2} m", d.driven),
          ];
          for (i, len) in d.directors.iter().enumerate() {
            parts.push(format!("引向器 {} ≈ {len:.2} m", i + 1));
          }
          parts.push(format!(
            "反射器–激励间距 ≈ {:.2} m · 引向器间距 ≈ {:.2} m",
            d.refl_spacing, d.dir_spacing
          ));
          parts.join("　")
        }}
      </div>
    </div>
  }
}

/// 垂直天线尺寸计算器。
#[component]
pub(super) fn VerticalCalculator() -> impl IntoView {
  let freq = RwSignal::new(14.2);
  let k = RwSignal::new(0.95);

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{t("频率（MHz）")}</span>
        <input
          type="number"
          step="0.01"
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
        <span class="text-xs text-muted-foreground">{t("缩短系数 k（0.5–1.0）")}</span>
        <input
          type="number"
          step="0.01"
          min="0.5"
          max="1.0"
          prop:value=move || k.get().to_string()
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              k.set(v);
            }
          }
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let Some((radiator, radial)) = vertical_dims(freq.get(), k.get()) else {
            return t("请输入有效频率与缩短系数");
          };
          format!("辐射体 ≈ {radiator:.2} m　地网（每根）≈ {radial:.2} m")
        }}
      </div>
    </div>
  }
}
