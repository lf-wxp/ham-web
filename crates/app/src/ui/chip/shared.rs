//! chip 一族共用的样式 token 与组上下文。

use leptos::prelude::*;

/// chip 的外观由 `style/pixel/controls.css` 的 `.pxl-chip` 提供，选中态跟着
/// `aria-checked` / `aria-pressed` 走：状态只有一个事实来源（无障碍属性），
/// 视觉不会和读屏看到的状态不一致，也不必在 Rust 里再拼一遍 `ON` / `OFF` 类名。
pub(super) const CHIP_BASE: &str = "pxl-chip";

/// [`Chip`](super::Chip) 与 [`ChipGroup`](super::ChipGroup) 之间的上下文。
#[derive(Clone, Copy)]
pub(super) struct ChipCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) disabled: Signal<bool>,
}
