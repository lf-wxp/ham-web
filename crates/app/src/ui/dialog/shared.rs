//! 弹层的共享状态属性（`data-state`）。
//!
//! 焦点编排、滚动锁与 Esc 都不在这里 —— 两套 Kit 共用 [`crate::focus`] 与
//! [`crate::overlay`]；「什么时候挂上去、什么时候摘下来」（延迟卸载）在 [`crate::presence`]。
//! 这里曾经各写一份（opener 记录、计数锁、window 监听、延迟卸载），于是现代风那边漏掉了
//! 滚动锁、像素风这边多出一份 Tab 陷阱；收拢成「一份原语 + 一份编排」之后才不会再漂移。

use leptos::prelude::*;

/// 退场期间的 `inert`（与 [`state_attr`] 配对）：关闭后的对话框还要留一个动画的时间，
/// 但读屏与键盘都不该再碰它。说明见 [`crate::ui::popover::inert`]。
pub(super) fn inert_attr(
  open: RwSignal<bool>,
) -> impl Fn() -> Option<&'static str> + Copy + Send + Sync {
  move || (!open.get()).then_some("")
}

/// 退场期间的 `aria-hidden`（与 [`inert_attr`] 并用）：说明见 [`crate::ui::popover::aria_hidden`]。
pub(super) fn aria_hidden_attr(
  open: RwSignal<bool>,
) -> impl Fn() -> Option<&'static str> + Copy + Send + Sync {
  move || (!open.get()).then_some("true")
}

pub(super) fn state_attr(open: RwSignal<bool>) -> impl Fn() -> &'static str + Copy + Send + Sync {
  move || if open.get() { "open" } else { "closed" }
}
