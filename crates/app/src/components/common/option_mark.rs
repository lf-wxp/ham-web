use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

/// 判分后一个选项行的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionVerdict {
  /// 还没判分，或这一行与结论无关。
  None,
  /// 这是正确选项。
  Right,
  /// 用户选了它，但它不是正确选项。
  Wrong,
}

/// 选项行尾的判分标记：像素 ✓ / ✗。
///
/// 为什么需要它：判分以前只有选项下方一行 `text-sm` 灰字，选项行本身没有任何对错标记，
/// 色觉障碍用户拿不到结果，读完整题再对照字母才知道错在哪。图标 + 行边色是冗余通道，
/// 不依赖颜色单独成立。图标本身 `aria-hidden`（`Icon` 的默认），结论由答案行的 live region
/// 播报，这里不再重复念一遍。
#[component]
pub fn OptionMark(#[prop(into)] verdict: Signal<OptionVerdict>) -> impl IntoView {
  move || match verdict.get() {
    OptionVerdict::None => ().into_any(),
    OptionVerdict::Right => {
      view! { <Icon kind=IconKind::Check class="ml-auto size-6 shrink-0 text-win-text" /> }
        .into_any()
    }
    OptionVerdict::Wrong => {
      view! { <Icon kind=IconKind::X class="ml-auto size-6 shrink-0 text-hp-text" /> }.into_any()
    }
  }
}
