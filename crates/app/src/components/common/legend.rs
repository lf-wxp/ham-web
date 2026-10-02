//! 图例：色块 + 文字的统一图例条（地图页、统计页通用）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;

/// 图例项：`(色块类名, 文字中文原文)`。
///
/// 色块类名需以完整字面量出现在调用方（供 Tailwind 扫描），
/// 文字走 [`t`] 按当前语言翻译（响应式）。
#[component]
pub fn Legend(items: Vec<(&'static str, &'static str)>) -> impl IntoView {
  view! {
    <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
      {items
        .into_iter()
        .map(|(swatch, label)| {
          view! {
            <span class="flex items-center gap-1.5">
              <span class=cn(&["inline-block", swatch])></span>
              <span>{move || t(label)}</span>
            </span>
          }
        })
        .collect_view()}
    </div>
  }
}
