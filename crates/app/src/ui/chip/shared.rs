//! chip 一族共用的样式 token 与组上下文。

use leptos::prelude::*;

/// chip 的基础类：现代风 `md-chip`（32px 胶囊、蓝底选中），像素风 `pxl-chip`。
///
/// 选中态跟着 `aria-checked` / `aria-pressed` 走：状态只有一个事实来源（无障碍属性），
/// 视觉不会和读屏看到的状态不一致，也不必在 Rust 里再拼一遍 `ON` / `OFF` 类名。
#[must_use]
pub(super) fn chip_base() -> &'static str {
  if crate::kit::is_modern_at_mount() {
    "md-chip"
  } else {
    "pxl-chip"
  }
}

/// [`Chip`](super::Chip) 与 [`ChipGroup`](super::ChipGroup) 之间的上下文。
#[derive(Clone, Copy)]
pub(super) struct ChipCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) disabled: Signal<bool>,
}
