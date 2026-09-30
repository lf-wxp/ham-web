use leptos::prelude::*;

use crate::cn::cn;

/// 对话框标题。
#[component]
pub fn DialogTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2 data-slot="dialog-title" class=cn(&["text-lg leading-none font-semibold", &class])>
      {children()}
    </h2>
  }
}
