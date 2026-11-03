//! `bar_row`：从 `awards_panel.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::i18n::t;

/// 一行「名称 · n / 目标」进度条。
pub(super) fn bar_row(label: String, n: usize, target: usize) -> impl IntoView {
  let pct = (n as f64 / target.max(1) as f64 * 100.0).min(100.0);
  let done = n >= target;
  // 现代风：与 `/stats`、`/log` 的条形图同一套轨道与渐变（达成后转绿）。
  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-award">
        <div class="md-award-head">
          <span>{label}</span>
          <span class="md-mono md-muted md-sm">
            {format!("{n} / {target}")}
            {done
              .then(|| {
                view! { <span class="md-award-done">{move || t("log.achieved")}</span> }
              })}
          </span>
        </div>
        <div class="md-bar-track">
          <span
            class=if done { "md-bar-fill md-bar-fill-done" } else { "md-bar-fill" }
            style=format!("width: {pct:.1}%")
          ></span>
        </div>
      </div>
    }
    .into_any();
  }
  view! {
    <div>
      <div class="mb-1 flex items-center justify-between text-xs">
        <span>{label}</span>
        <span class="tabular-nums text-muted-foreground">
          {format!("{n} / {target}")}
          {done.then(|| view! { <span class="ml-1.5 text-win-text">{move || t("log.achieved")}</span> })}
        </span>
      </div>
      <div class="h-2 w-full overflow-hidden bg-muted">
        <div
          class=if done { "h-full bg-win/70" } else { "h-full bg-primary" }
          style=format!("width: {pct:.1}%")
        ></div>
      </div>
    </div>
  }
  .into_any()
}
