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
  // 现代风：圆形单选（`md-check md-radio`），选中是橙底 + 内圈白点。
  let modern = crate::kit::is_modern_at_mount();
  let class = if modern {
    cn(&["md-check md-radio shrink-0", &class])
  } else {
    cn(&["pxl-check pxl-radio shrink-0", &class])
  };
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
                // 现代风的选中动画是平滑的 `md-check-pop`；`motion-check` 是像素风的逐格跳。
                class=if modern {
                  "md-check-pop relative flex items-center justify-center"
                } else {
                  "motion-check relative flex items-center justify-center"
                }
              >
                <span class=if modern { "md-radio-dot" } else { "pxl-radio-dot" }></span>
              </span>
            }
          })
      }}
    </button>
  }
}
