//! `bar_row`：从 `awards_panel.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::i18n::t;

/// 一行「名称 · n / 目标」进度条。
pub(super) fn bar_row(label: String, n: usize, target: usize) -> impl IntoView {
  let pct = (n as f64 / target.max(1) as f64 * 100.0).min(100.0);
  let done = n >= target;
  view! {
    <div>
      <div class="mb-1 flex items-center justify-between text-xs">
        <span class="font-medium">{label}</span>
        <span class="tabular-nums text-muted-foreground">
          {format!("{n} / {target}")}
          {done.then(|| view! { <span class="ml-1.5 font-semibold text-emerald-600 dark:text-emerald-400">{move || t("log.achieved")}</span> })}
        </span>
      </div>
      <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          class=if done { "h-full rounded-full bg-emerald-500" } else { "h-full rounded-full bg-primary" }
          style=format!("width: {pct:.1}%")
        ></div>
      </div>
    </div>
  }
}
