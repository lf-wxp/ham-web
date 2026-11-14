use leptos::html;
use leptos::prelude::*;

use super::readout::Readout;

/// 统计卡片：大数字 + 标签。`value` 可为静态 `usize` 或响应式 `Signal<usize>`；
/// `label` 要跟随语言切换就传 `Signal::derive(move || t("…"))`（静态 `&str` / `String` 也收）。
///
/// `value` 刻意是 `usize` 而不是格式化好的字符串：像素风的数字是一块七段屏，只画得了
/// `rig_panel::segment_mask` 认得的字符 —— 传「1,234」或带单位的文案会被画成全灭的格子，
/// 屏幕上看起来是一片空白。要加单位 / 千分位就放在 `label` 里，别改这里的类型。
///
/// 带 `motion-lift`（悬停抬起）并参与滚动浮现：统计格通常是「整块可扫读」的信息，
/// 逐个浮现比整页一次性出现更容易抓住注意力。
///
/// 数值变化时做一次 expo-out 的数字滚动（尊重「减少动态效果」偏好），
/// 让「答对后计数 +1」这类变化有明确的视觉反馈，而不是干巴巴地跳数字。
#[component]
pub fn Stat(
  #[prop(into)] label: Signal<String>,
  #[prop(into)] value: Signal<usize>,
) -> impl IntoView {
  let node = NodeRef::<html::Div>::new();
  node.on_load(|el| crate::motion::reveal_on_scroll(&el, 0));

  // 展示值：与 `value` 分离，滚动时逐步逼近，避免直接跳变（缓动在 `core::count_up`，钩子在 `crate::motion`）。
  let display = crate::motion::use_count_up(value, false);

  if crate::kit::is_modern_at_mount() {
    return view! {
      <div node_ref=node class="md-stat">
        <div class="md-stat-value md-mono">{move || display.get()}</div>
        <div class="md-stat-label">{move || label.get()}</div>
      </div>
    }
    .into_any();
  }
  // 像素风的数字是一块小读数屏（七段，`docs/ui/pixel.md` §3.4）。七段图形对读屏是装饰，
  // 数值另放一份 `sr-only` 文本：读屏与按文字定位的 e2e 读到的仍是那串数字。
  view! {
    <div node_ref=node class="pxl-window motion-lift p-4">
      <div class="pxl-display inline-flex px-2 py-1.5" aria-hidden="true">
        <Readout text=Signal::derive(move || display.get().to_string()) scale=3 />
      </div>
      <span class="sr-only">{move || display.get()}</span>
      <div class="mt-1.5 text-xs text-muted-foreground">{move || label.get()}</div>
    </div>
  }
  .into_any()
}
