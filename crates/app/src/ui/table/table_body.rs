//! [`TableBody`]：表格主体（`<tbody>`）。

use leptos::prelude::*;

/// 表格主体。行悬停在 `.pxl-table tbody tr` 统一给，这里只保留语义；
/// 等宽数字等整体修饰用 `class`（如 `class="font-mono"`）。
#[component]
pub fn TableBody(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! { <tbody data-slot="table-body" class=class>{children()}</tbody> }
}
