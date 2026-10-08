use leptos::prelude::*;

use crate::cn::cn;

use super::super::control::TextValue;

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
  /// 整组的无障碍名称（每组互斥选项都该有）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
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
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      class=cn(&["grid gap-3", &class])
      data-disabled=move || disabled.get().then_some("")
    >
      {children()}
    </div>
  }
}
