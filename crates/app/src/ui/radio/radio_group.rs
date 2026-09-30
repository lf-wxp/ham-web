use leptos::prelude::*;

use crate::cn::cn;

#[derive(Clone, Copy)]
pub(super) struct RadioCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) disabled: Signal<bool>,
}

#[component]
pub fn RadioGroup(
  #[prop(into)] value: Signal<String>,
  on_change: Callback<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
  children: Children,
) -> impl IntoView {
  provide_context(RadioCtx {
    value,
    on_change,
    disabled,
  });
  view! {
    <div
      role="radiogroup"
      data-slot="radio-group"
      class=cn(&["grid gap-3", &class])
      data-disabled=move || disabled.get().then_some("")
    >
      {children()}
    </div>
  }
}
