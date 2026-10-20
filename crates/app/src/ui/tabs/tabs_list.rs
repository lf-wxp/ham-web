//! [`TabsList`]：标签栏（`role="tablist"`）。

use leptos::prelude::*;

use crate::cn::cn;

use super::super::control::TextValue;

/// 标签栏：把若干 [`TabsTrigger`](super::TabsTrigger) 包成一组。
///
/// 必须放在 [`Tabs`](super::Tabs) 的 `children` 里就地构建。
#[component]
pub fn TabsList(
  /// 整组的无障碍名称。一个页面上只有一组 tab 时可省，多组时必须给。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 覆盖默认外观（默认 `pxl-tablist`：底部墨线 + 折行）。
  #[prop(optional, into)]
  class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <div
      role="tablist"
      data-slot="tabs-list"
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      class=cn(&["pxl-tablist", &class])
    >
      {children()}
    </div>
  }
}
