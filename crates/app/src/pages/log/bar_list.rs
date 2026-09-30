use leptos::prelude::*;

/// 统计条形图（名称 → 次数，按最大值比例绘制）。
#[component]
pub(super) fn BarList(items: Vec<(String, usize)>) -> impl IntoView {
  let max = items.iter().map(|(_, n)| *n).max().unwrap_or(1);
  view! {
    <div class="space-y-2">
      {items
        .iter()
        .map(|(k, n)| {
          let pct = (*n as f64 / max as f64 * 100.0).round() as i32;
          let k = k.clone();
          let n = *n;
          view! {
            <div class="flex items-center gap-2 text-xs">
              <span class="w-12 shrink-0 font-mono text-muted-foreground">{k}</span>
              <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                <div class="h-2 rounded-full bg-primary" style=format!("width: {pct}%")></div>
              </div>
              <span class="w-6 shrink-0 text-right tabular-nums text-muted-foreground">{n}</span>
            </div>
          }
        })
        .collect_view()}
    </div>
  }
}
