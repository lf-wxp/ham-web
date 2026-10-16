//! [`TableCell`]：数据单元格（`<td>`）。

use leptos::prelude::*;

use crate::ui::TextValue;

/// 数据单元格。对齐 / 字号 / 颜色等修饰写在 `class`（后写胜出）。
/// 跨列空态行（「暂无数据」）传 `colspan`；装饰性单元格传 `aria_hidden=true`。
#[component]
pub fn TableCell(
  #[prop(optional, into)] class: String,
  #[prop(optional)] colspan: Option<u32>,
  #[prop(optional)] aria_hidden: bool,
  #[prop(optional, into)] title: Option<TextValue>,
  children: Children,
) -> impl IntoView {
  view! {
    <td
      data-slot="table-cell"
      class=class
      colspan=colspan
      aria-hidden=aria_hidden.then_some("true")
      title=move || title.as_ref().map(TextValue::get)
    >
      {children()}
    </td>
  }
}
