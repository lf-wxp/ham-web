use leptos::prelude::*;

use crate::ui::Tooltip;

/// 水平条形图。
#[component]
pub(super) fn BarChart(items: Vec<(String, usize)>, max: usize) -> impl IntoView {
  let max = max.max(1) as f64;
  // 现代风：细条 + 渐变填充 + 圆角，标签与计数用等宽数字。
  if crate::kit::is_modern_at_mount() {
    let rows: Vec<_> = items
      .into_iter()
      .map(|(label, count)| {
        let w = count as f64 / max * 100.0;
        (label, count, w)
      })
      .collect();
    return view! {
      <div class="md-bars">
        {rows
          .into_iter()
          .map(|(label, count, w)| {
            let title = label.clone();
            view! {
              <div class="md-bar-row">
                <span class="md-bar-label" title=title>{label}</span>
                <span class="md-bar-track">
                  <span class="md-bar-fill" style=format!("width: {w:.1}%")></span>
                </span>
                <span class="md-bar-count">{count}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    }
    .into_any();
  }
  view! {
    <div class="space-y-1.5">
      {items
        .into_iter()
        .map(|(label, count)| {
          let w = count as f64 / max * 100.0;
          // 标签可能被截断，`title` 保留完整文本（各语言下都能看到全名）。
          let label_title = label.clone();
          view! {
            <div class="flex items-center gap-2 text-sm">
              <Tooltip class="w-28 shrink-0 sm:w-40 lg:w-56" label=label_title>
                <span class="w-full truncate text-xs text-muted-foreground">
                  {move || label.clone()}
                </span>
              </Tooltip>
              <div class="h-4 flex-1 overflow-hidden bg-muted">
                <div class="h-full bg-primary/70" style=format!("width: {w:.1}%")></div>
              </div>
              <span class="w-10 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                {count}
              </span>
            </div>
          }
        })
        .collect_view()}
    </div>
  }
  .into_any()
}
