use leptos::prelude::*;

use super::{INPUT, RESULT, fmt_num};
use crate::i18n::{t, tf};

/// 级联增益：多个 dB 相加（用逗号或空格分隔）。
#[component]
pub(super) fn CascadeGain() -> impl IntoView {
  let input = RwSignal::new("3, 6".to_owned());

  view! {
    <div class="space-y-3">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("各级增益（dB，用逗号或空格分隔）")}</span>
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
            .collect();
          let sum: f64 = values.iter().sum();
          tf(
            "{} 级，级联增益 = {} dB，总功率倍数 ×{}",
            &[
              &values.len().to_string(),
              &fmt_num(sum),
              &fmt_num(10f64.powf(sum / 10.0)),
            ],
          )
        }}
      </div>
    </div>
  }
}
