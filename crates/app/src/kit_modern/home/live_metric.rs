//! 实时传播面板里的一格读数（SFI / A / K）。
//!
//! 与 `ModernStatValue` 分开：那个是「累计量」，要滚动起跳、字号到 56px；
//! 这里是「外部世界的当前值」，加载中显示破折号而不是 0（0 会被误读成真实读数）。
//!
//! 加载中改用一块骨架（见 `docs/ui/modern.md` §4.3）：原来从「—」跳到真值时，字宽从 1 个字符
//! 变成 2–3 个字符，三格读数会一起左右挪一下 —— 首页首屏最显眼的位置不该有这种抖动。
//! 骨架宽度按「三位数字」预留，真值到位时占位与数字同宽。

use leptos::prelude::*;

use crate::i18n::t;

#[component]
pub(super) fn LiveMetric(
  label: &'static str,
  value: Signal<Option<i32>>,
  /// 请求进行中：为真且还没有值时画骨架，否则画破折号（「没有这个指数」与「还没到」是两件事）。
  #[prop(optional, into)]
  loading: Signal<bool>,
) -> impl IntoView {
  view! {
    <div class="md-live-metric">
      {move || match (value.get(), loading.get()) {
        (Some(v), _) => view! { <span class="md-live-metric-value">{v.to_string()}</span> }.into_any(),
        (None, true) => {
          view! {
            <span class="md-live-metric-value">
              <span class="md-skel md-live-metric-skel" aria-hidden="true"></span>
              <span class="sr-only">{move || t("learning.loading")}</span>
            </span>
          }
            .into_any()
        }
        (None, false) => view! { <span class="md-live-metric-value">"—"</span> }.into_any(),
      }}
      <span class="md-live-metric-label">{move || t(label)}</span>
    </div>
  }
}
