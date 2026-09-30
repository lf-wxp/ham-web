use leptos::prelude::*;

use super::label_class;

/// 表单标签。
#[component]
pub fn Label(
  #[prop(into)] r#for: String,
  #[prop(optional, into)] class: Signal<String>,
  children: Children,
) -> impl IntoView {
  view! {
    <label data-slot="label" for=r#for class=move || label_class(&class.get())>
      {children()}
    </label>
  }
}
