//! [`ScrollRegion`]：包着裸 `<table>` 的横向滚动容器，溢出时键盘可达。

use leptos::prelude::*;

use crate::ui::TextValue;

use super::scroll_overflow::use_scroll_overflow;

/// 横向滚动容器，给**不走 [`Table`](super::Table)** 的裸表格用（现代风的知识页表格与速查页表格，
/// 它们的版式由 `.md-panel table:not(.md-table)` / `.md-table` 单独承担）。
///
/// 键盘可达口径与 `Table` 一致：容器溢出时自动加 `tabindex="0"` 与 `role="region"`，
/// 不溢出时不进 Tab 序（WCAG 2.1.1）。`aria_label` 只在溢出（有区域语义）时才挂到容器上 ——
/// `aria-label` 挂在无 role 的 `div` 上会被 axe 报 `aria-prohibited-attr`（`Table` 的容器同一口径）。
/// 滚动本身的样式（`overflow-x` 等）由调用方用 `class` 给，例如现代风的 `md-table-scroll`。
///
/// **第一个子元素必须就是那张 `<table>`**：溢出检测除了容器还观察它的第一个子元素 ——
/// 只看容器的话，列宽变化（文案折行、`min-width` 生效）不会触发回调。表格外再包一层 `div`，
/// 这里观察到的就是那层 `div`，同样的变化会漏报。
#[component]
pub fn ScrollRegion(
  #[prop(optional, into)] aria_label: Option<TextValue>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let container = NodeRef::<leptos::html::Div>::new();
  let scrollable = use_scroll_overflow(container);
  view! {
    <div
      node_ref=container
      class=class
      tabindex=move || scrollable.get().then_some("0")
      role=move || scrollable.get().then_some("region")
      aria-label=move || {
        scrollable.get().then(|| aria_label.as_ref().map(TextValue::get)).flatten()
      }
    >
      {children()}
    </div>
  }
}
