use leptos::prelude::*;

/// 统计条形图（名称 → 次数，按最大值比例绘制）。
#[component]
pub(super) fn BarList(items: Vec<(String, usize)>) -> impl IntoView {
  let max = items.iter().map(|(_, n)| *n).max().unwrap_or(1);
  // 现代风：与 `/stats` 的 `BarChart` 同一套（圆角轨道 + 渐变填充），
  // 否则同一个页面里会出现两种条形图。
  if crate::kit::is_modern_at_mount() {
    let rows: Vec<(String, usize, f64)> = items
      .iter()
      .map(|(k, n)| (k.clone(), *n, *n as f64 / max as f64 * 100.0))
      .collect();
    return view! {
      <div class="md-bars">
        {rows
          .into_iter()
          .map(|(k, n, pct)| {
            view! {
              <div class="md-bar-row">
                <span class="md-bar-label md-mono">{k}</span>
                <span class="md-bar-track">
                  <span class="md-bar-fill" style=format!("width: {pct:.1}%")></span>
                </span>
                <span class="md-bar-count">{n}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    }
    .into_any();
  }
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
              <div class="h-2 flex-1 overflow-hidden bg-muted">
                <div class="h-2 bg-primary" style=format!("width: {pct}%")></div>
              </div>
              <span class="w-6 shrink-0 text-right tabular-nums text-muted-foreground">{n}</span>
            </div>
          }
        })
        .collect_view()}
    </div>
  }
  .into_any()
}
