//! Lucide 图标（与原项目 lucide-react@0.539 的 SVG 路径一致）。

mod icon_kind;
mod icon_lookup;

pub use icon_kind::IconKind;
pub use icon_lookup::icon_of;

use leptos::prelude::*;

/// 渲染一个 Lucide 图标。
#[component]
pub fn Icon(kind: IconKind, #[prop(optional, into)] class: Signal<String>) -> impl IntoView {
  let class = move || format!("lucide lucide-{} {}", kind.name(), class.get());
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class=class
      aria-hidden="true"
      inner_html=kind.body()
    ></svg>
  }
}
