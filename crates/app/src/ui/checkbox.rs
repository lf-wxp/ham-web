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
  let class = cn(&[
    "peer border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive size-4 shrink-0 rounded-[5px] border shadow-xs transition-[box-shadow,background-color,border-color] outline-none focus-visible:ring-[3px] data-[state=checked]:shadow-[0_0_10px_-2px_var(--primary)] disabled:cursor-not-allowed disabled:opacity-50",
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
                <Icon kind=IconKind::Check class="size-3.5" />
              </span>
            }
          })
      }}
    </button>
  }
}
