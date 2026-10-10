//! [`ChipToggle`]：筛选条上的独立开关（可以一个都不选）。

use leptos::prelude::*;

use crate::cn::cn;

use super::super::control::TextValue;
use super::shared::CHIP_BASE;

/// 单个 chip 开关（筛选条上的「只看需要的」「提醒」这类独立开关）。
///
/// 与 [`Chip`](super::Chip) 的区别是语义：这里是独立的开 / 关（`aria-pressed`），
/// 而 [`Chip`](super::Chip) 属于互斥组（`role="radio"`）。
#[component]
pub fn ChipToggle(
  #[prop(into)] active: Signal<bool>,
  on_change: Callback<bool>,
  /// 无可见文字时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 悬停提示（原生 `title`）。
  #[prop(optional, into)]
  title: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let class = cn(&[CHIP_BASE, &class]);
  view! {
    <button
      type="button"
      data-slot="chip-toggle"
      aria-pressed=move || active.get().to_string()
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      title=move || title.as_ref().map(TextValue::get)
      disabled=move || disabled.get()
      class=class
      on:click=move |_| {
        if !disabled.get_untracked() {
          on_change.run(!active.get_untracked());
        }
      }
    >
      {children()}
    </button>
  }
}
