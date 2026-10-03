//! Yagi 天线振子尺寸计算器。

use ham_web_core::antenna_design::yagi_dims;
use leptos::prelude::*;

use crate::i18n::{t, tf};

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
            tf("反射器 ≈ {} m", &[&format!("{:.2}", d.reflector)]),
            tf("激励振子 ≈ {} m", &[&format!("{:.2}", d.driven)]),
          ];
          for (i, len) in d.directors.iter().enumerate() {
            parts.push(tf(
              "引向器 {} ≈ {} m",
              &[&(i + 1).to_string(), &format!("{:.2}", len)],
            ));
          }
          parts.push(tf(
            "反射器–激励间距 ≈ {} m · 引向器间距 ≈ {} m",
            &[
              &format!("{:.2}", d.refl_spacing),
              &format!("{:.2}", d.dir_spacing),
            ],
          ));
          parts.join("　")
        }}
      </div>
    </div>
  }
}
