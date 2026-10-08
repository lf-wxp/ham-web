//! [`ChipGroup`]：把一组 [`Chip`](super::Chip) 包成互斥选择。

use leptos::prelude::*;

use crate::cn::cn;

use super::super::control::TextValue;
use super::shared::ChipCtx;

/// 分段选择组。
#[component]
pub fn ChipGroup(
  /// 当前选中的值。
  #[prop(into)]
  value: Signal<String>,
  on_change: Callback<String>,
  /// 整组的无障碍名称（旁边没有可见说明文字时必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  provide_context(ChipCtx {
    value,
    on_change,
    disabled,
  });
  view! {
    <div
      role="radiogroup"
      data-slot="chip-group"
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      data-disabled=move || disabled.get().then_some("")
      class=cn(&["flex flex-wrap items-center gap-2", &class])
    >
      {children()}
    </div>
  }
}
