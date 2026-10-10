//! chip 一族共用的样式 token 与组上下文。

use leptos::prelude::*;

/// chip 的公共外观（尺寸、圆角、过渡）。胶囊形：和按钮的圆角矩形区分开，一眼看出「这是个标签」。
pub(super) const CHIP_BASE: &str = "shrink-0 cursor-pointer whitespace-nowrap rounded-full border px-3 py-1 text-xs font-medium transition-[color,background-color,border-color,box-shadow] duration-200 outline-none focus-visible:border-ring focus-visible:ring-ring/40 focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50";
/// 选中态：实心 + 一圈带色的柔光，在一排未选中项里一眼就能找到当前项。
pub(super) const CHIP_ON: &str =
  "border-primary bg-primary text-primary-foreground shadow-sm shadow-primary/30";
/// 未选中态。
pub(super) const CHIP_OFF: &str = "bg-background/60 text-muted-foreground hover:border-primary/40 hover:bg-accent hover:text-accent-foreground";

/// [`Chip`](super::Chip) 与 [`ChipGroup`](super::ChipGroup) 之间的上下文。
#[derive(Clone, Copy)]
pub(super) struct ChipCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) disabled: Signal<bool>,
}
