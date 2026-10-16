//! [`TableRow`]：表格行（`<tr>`）。

use leptos::prelude::*;

/// 表格行。行悬停 / 过渡在 `.pxl-table tbody tr` 统一给，调用点用 `class`
/// 覆盖特殊行的强调（如汇总行）。
#[component]
pub fn TableRow(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! { <tr data-slot="table-row" class=class>{children()}</tr> }
}
