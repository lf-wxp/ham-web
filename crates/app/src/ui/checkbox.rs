//! 复选框（对应 Radix Checkbox）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::control::TextValue;

#[component]
pub fn Checkbox(
  #[prop(into)] checked: Signal<bool>,
  on_change: Callback<bool>,
  #[prop(optional, into)] id: Option<String>,
  /// 无可见标签时的无障碍名称（表格里逐行的勾选必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&["pxl-check peer", &class]);
  view! {
    <button
      type="button"
      role="checkbox"
      data-slot="checkbox"
      id=id
      class=class
      aria-checked=move || checked.get().to_string()
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      data-state=move || if checked.get() { "checked" } else { "unchecked" }
      data-disabled=move || disabled.get().then_some("")
      disabled=move || disabled.get()
      on:click=move |_| {
        if !disabled.get_untracked() {
          on_change.run(!checked.get_untracked());
        }
      }
    >
      {move || {
        checked
          .get()
          .then(|| {
            view! {
              <span
                data-slot="checkbox-indicator"
                data-state="checked"
                class="motion-check flex items-center justify-center text-current"
                style="pointer-events: none;"
              >
                <Icon kind=IconKind::Check class="size-4" />
              </span>
            }
          })
      }}
    </button>
  }
}
