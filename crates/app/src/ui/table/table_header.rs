//! [`TableHeader`]：表头单元格（`<th>`）。

use leptos::prelude::*;

/// 表头单元格。首列作为行标题时传 `scope="row"`，列标题是默认（CSS 里
/// `.pxl-table :is(th, td)` 统一左对齐，列标题要居中另写 `class`）。
/// 分组标题跨多列时传 `colspan`。
#[component]
pub fn TableHeader(
  #[prop(optional)] scope: Option<&'static str>,
  #[prop(optional)] colspan: Option<u32>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! { <th data-slot="table-header" scope=scope colspan=colspan class=class>{children()}</th> }
}
