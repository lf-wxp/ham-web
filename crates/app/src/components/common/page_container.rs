use leptos::prelude::*;

use crate::cn::cn;

/// 页面内容容器：统一内容页的最大宽度与内边距（`max-w-5xl`）。
///
/// 默认带 `space-y-6` 以纵向分隔内容块，可用 `class` 追加或覆盖间距。
#[component]
pub fn PageContainer(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <div class=cn(&["mx-auto max-w-5xl space-y-6 px-4 py-5", &class])>
      {children()}
    </div>
  }
}
