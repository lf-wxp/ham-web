//! 状态一览：8 档切换 + 8 行 ARIA 对照 + 整组控件联动。
//!
//! 这块的档位与选择器是两套皮肤共用的契约，直接复用同一份「状态试验台」，
//! 项目与交互逐项对齐，控件在 `crate::ui` 里自动派发成 `md-*`。

use leptos::prelude::*;

use super::super::states_section::StatesSection;

#[component]
pub fn ModernStates() -> impl IntoView {
  view! { <StatesSection /> }
}
