use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};
use crate::i18n::{t, tf};

/// 电阻串并联：输入若干电阻，计算串联与并联等效（Ω）。
#[component]
pub(super) fn ResistorParallel() -> impl IntoView {
  let input = RwSignal::new("100, 100".to_owned());

  view! {
    <div class="space-y-3">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("电阻值（Ω，用逗号或空格分隔）")}</span>
        <input
          prop:value=move || input.get()
          on:input=move |e| input.set(event_target_value(&e))
          class=INPUT
        />
      </label>
      <div class=RESULT>
        {move || {
          let values: Vec<f64> = input
            .get()
            .split([',', '，', ' '])
            .filter_map(|s| s.trim().parse().ok())
            .filter(|v| *v > 0.0)
            .collect();
          if values.is_empty() {
            t("请输入电阻值")
          } else {
            let series: f64 = values.iter().sum();
            let parallel = 1.0 / values.iter().map(|v| 1.0 / v).sum::<f64>();
            tf(
              "串联 = {} Ω，并联 = {} Ω",
              &[&fmt_num(series), &fmt_num(parallel)],
            )
          }
        }}
      </div>
    </div>
  }
}
