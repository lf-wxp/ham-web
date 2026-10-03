use leptos::prelude::*;

/// 水平条形图。
#[component]
pub(super) fn BarChart(items: Vec<(String, usize)>, max: usize) -> impl IntoView {
  let max = max.max(1) as f64;
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
              <span
                class="w-28 shrink-0 truncate text-xs text-muted-foreground sm:w-40 lg:w-56"
                title=label_title
              >
                {move || label.clone()}
              </span>
              <div class="h-4 flex-1 overflow-hidden rounded bg-muted">
                <div class="h-full rounded bg-primary/70" style=format!("width: {w:.1}%")></div>
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
}
