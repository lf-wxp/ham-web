//! 实时传播面板里的一个波段格（如 `80m-40m` + 条件）。
//!
//! 条件原文是 HamQSL 的英文（Good / Fair / Poor），这里只负责渲染：
//! 文案 key 由调用方映射（`home.good` 等），色调由 `data-tone` 交给 CSS，
//! 免得在组件里出现第二套配色。

use leptos::prelude::*;

use crate::i18n::t;

#[component]
pub(super) fn LiveBand(
  /// 波段名（接口原样，如 `80m-40m`）。
  band: String,
  /// `good` / `warn` / `muted` / `unknown`，由 CSS 决定色条颜色。
  tone: &'static str,
  /// 条件文案的 i18n key；接口给了不认识的值时为 `None`。
  label: Option<&'static str>,
) -> impl IntoView {
  view! {
    <div class="md-live-band" data-tone=tone>
      <span class="md-live-band-bar" aria-hidden="true"></span>
      <span class="md-live-band-name">{band}</span>
      <span class="md-live-band-state">
        {move || label.map_or_else(|| "—".to_owned(), t)}
      </span>
    </div>
  }
}
