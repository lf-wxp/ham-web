use leptos::prelude::*;

use crate::cn::cn;

/// 对话框标题。
#[component]
pub fn DialogTitle(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  view! {
    <h2
      data-slot="dialog-title"
      class=if crate::kit::is_modern_at_mount() {
        cn(&["md-dialog-title", &class])
      } else {
        cn(&["pxl-title text-base leading-snug pr-10", &class])
      }
    >
      {children()}
    </h2>
  }
}
