//! chip 一族共用的样式 token 与组上下文。

use leptos::prelude::*;

/// chip 的公共外观（尺寸、圆角、过渡）。
pub(super) const CHIP_BASE: &str = "shrink-0 cursor-pointer whitespace-nowrap rounded-md border px-3 py-1 text-xs font-medium transition-colors outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50";
/// 选中态。
pub(super) const CHIP_ON: &str = "border-primary bg-primary text-primary-foreground";
/// 未选中态。
pub(super) const CHIP_OFF: &str =
  "bg-background text-muted-foreground hover:bg-accent hover:text-accent-foreground";

/// [`Chip`](super::Chip) 与 [`ChipGroup`](super::ChipGroup) 之间的上下文。
#[derive(Clone, Copy)]
pub(super) struct ChipCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) disabled: Signal<bool>,
}
