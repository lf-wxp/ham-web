//! 单选组（对应 Radix RadioGroup）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

#[derive(Clone, Copy)]
struct RadioCtx {
  value: Signal<String>,
  on_change: Callback<String>,
  disabled: Signal<bool>,
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

#[component]
pub fn RadioGroupItem(
  #[prop(into)] value: String,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let ctx = expect_context::<RadioCtx>();
  let class = cn(&[
    "border-input text-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:bg-input/30 aspect-square size-4 shrink-0 rounded-full border shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
    &class,
  ]);
  let v = StoredValue::new(value);
  let checked = move || ctx.value.with(|cur| v.with_value(|v| cur == v));
  let is_disabled = move || ctx.disabled.get() || disabled.get();
  view! {
    <button
      type="button"
      role="radio"
      data-slot="radio-group-item"
      id=id
      value=v.get_value()
      class=class
      aria-checked=move || checked().to_string()
      data-state=move || if checked() { "checked" } else { "unchecked" }
      disabled=is_disabled
      on:click=move |_| {
        if !is_disabled() && !checked() {
          ctx.on_change.run(v.get_value());
        }
      }
    >
      {move || {
        checked()
          .then(|| {
            view! {
              <span data-slot="radio-group-indicator" data-state="checked" class="relative flex items-center justify-center">
                <Icon
                  kind=IconKind::Circle
                  class="fill-primary absolute top-1/2 left-1/2 size-2 -translate-x-1/2 -translate-y-1/2"
                />
              </span>
            }
          })
      }}
    </button>
  }
}
