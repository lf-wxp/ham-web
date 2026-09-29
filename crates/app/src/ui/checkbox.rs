//! 复选框（对应 Radix Checkbox）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

#[component]
pub fn Checkbox(
  #[prop(into)] checked: Signal<bool>,
  on_change: Callback<bool>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&[
    "peer border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive size-4 shrink-0 rounded-[4px] border shadow-xs transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
    &class,
  ]);
  view! {
    <button
      type="button"
      role="checkbox"
      data-slot="checkbox"
      id=id
      class=class
      aria-checked=move || checked.get().to_string()
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
                class="flex items-center justify-center text-current transition-none"
                style="pointer-events: none;"
              >
                <Icon kind=IconKind::Check class="size-3.5" />
              </span>
            }
          })
      }}
    </button>
  }
}
