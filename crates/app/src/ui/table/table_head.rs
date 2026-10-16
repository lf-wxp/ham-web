//! [`TableHead`]：表头行组（`<thead>`）。

use leptos::prelude::*;

/// 表头行组。底纹与小字号由 `.pxl-table thead` 统一给，这里只保留语义；
/// 需要粘性表头等特殊形态时用 `class` 覆盖（如 `class="sticky top-0 bg-card"`）。
#[component]
pub fn TableHead(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! { <thead data-slot="table-head" class=class>{children()}</thead> }
}
