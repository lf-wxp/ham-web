use leptos::prelude::*;

use crate::cn::cn;

use super::radio_group::RadioCtx;

#[component]
pub fn RadioGroupItem(
  #[prop(into)] value: String,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let ctx = expect_context::<RadioCtx>();
  let class = cn(&["pxl-check pxl-radio shrink-0", &class]);
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
              <span
                data-slot="radio-group-indicator"
                data-state="checked"
                class="motion-check relative flex items-center justify-center"
              >
                <span class="pxl-radio-dot"></span>
              </span>
            }
          })
      }}
    </button>
  }
}
