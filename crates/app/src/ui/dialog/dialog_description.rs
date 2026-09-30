use leptos::prelude::*;

use crate::cn::cn;

/// 对话框描述。
#[component]
pub fn DialogDescription(
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  view! {
    <p data-slot="dialog-description" class=cn(&["text-muted-foreground text-sm", &class])>
      {children()}
    </p>
  }
}
