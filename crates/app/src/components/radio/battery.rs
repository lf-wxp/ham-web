use leptos::prelude::*;

use crate::ui::TextValue;

/// 电量格：一格一点电（抗干扰页「本台电量」），剩两格以下变琥珀、剩一格变红。
///
/// 仍是 `role="progressbar"`：它就是一个「还剩多少」的进度，读屏念百分比；格数的文字版本
/// （「电量 4 / 5」）由调用方另外给。
#[component]
pub fn Battery(
  #[prop(into)] level: Signal<u32>,
  max: u32,
  #[prop(into)] label: TextValue,
) -> impl IntoView {
  let max = max.max(1);
  let tier = move || match level.get() {
    0 | 1 => "low",
    2 => "mid",
    _ => "ok",
  };
  view! {
    <span
      class="pxl-battery"
      role="progressbar"
      aria-label=move || label.get()
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=move || (level.get().min(max) * 100 / max).to_string()
      data-tier=tier
    >
      {(0..max)
        .map(|i| view! { <span data-lit=move || (i < level.get()).then_some("")></span> })
        .collect_view()}
    </span>
  }
}
