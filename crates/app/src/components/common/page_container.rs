use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;

/// 页面内容容器：统一内容页的最大宽度与内边距（`max-w-5xl`）。
///
/// 默认带 `space-y-6` 以纵向分隔内容块，可用 `class` 追加或覆盖间距。
///
/// `reveal=true` 时，直接子元素会在滚动进入视口时逐个浮现（同批进入的按 60ms 错峰）。
/// 这是全站最主要的滚动触发动效；[`crate::motion::reveal_children`] 只取 `:scope > *`，
/// 所以嵌套层级各自决定自己的节奏，不会被一次性点亮。
#[component]
pub fn PageContainer(
  #[prop(optional, into)] class: String,
  #[prop(optional)] reveal: bool,
  children: Children,
) -> impl IntoView {
  let node = NodeRef::<html::Div>::new();
  node.on_load(move |el| {
    if reveal {
      crate::motion::reveal_children(&el);
    }
  });
  view! {
    <div node_ref=node class=cn(&["mx-auto max-w-5xl space-y-6 px-4 py-5", &class])>
      {children()}
    </div>
  }
}
