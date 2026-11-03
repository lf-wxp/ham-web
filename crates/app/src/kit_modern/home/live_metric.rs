//! 实时传播面板里的一格读数（SFI / A / K）。
//!
//! 与 `ModernStatValue` 分开：那个是「累计量」，要滚动起跳、字号到 56px；
//! 这里是「外部世界的当前值」，加载中显示破折号而不是 0（0 会被误读成真实读数）。

use leptos::prelude::*;

use crate::i18n::t;

#[component]
pub(super) fn LiveMetric(label: &'static str, value: Signal<Option<i32>>) -> impl IntoView {
  view! {
    <div class="md-live-metric">
      <span class="md-live-metric-value">
        {move || value.get().map_or_else(|| "—".to_owned(), |v| v.to_string())}
      </span>
      <span class="md-live-metric-label">{move || t(label)}</span>
    </div>
  }
}
