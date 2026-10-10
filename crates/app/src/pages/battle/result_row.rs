//! 结算窗口里的一行统计：左边说明，右边数值，中间一条虚线。

use leptos::prelude::*;

/// 一行统计。
#[component]
pub(super) fn ResultRow(#[prop(into)] label: Signal<String>, value: String) -> impl IntoView {
  view! {
    <div class="flex items-baseline justify-between gap-3 border-b-2 border-dashed border-border py-1">
      <dt class="text-sm text-muted-foreground">{move || label.get()}</dt>
      <dd class="pxl-label text-sm tabular-nums">{value}</dd>
    </div>
  }
}
