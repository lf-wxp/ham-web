//! 开关：替代原生 `<input type="checkbox">` 用于「立即生效」的设置项。
//!
//! 与 [`crate::ui::Checkbox`] 的分工：表单里需要提交 / 校验的勾选用 Checkbox，
//! 设置面板里「切换即生效」的用 Switch（语义上是按钮而非表单项）。

use leptos::prelude::*;

use crate::cn::cn;

use super::control::{CONTROL_RING, TextValue, invalid_attr};

/// 开关。
#[component]
pub fn Switch(
  /// 是否打开（受控）。
  #[prop(into)]
  checked: Signal<bool>,
  /// 切换后的新状态。
  on_change: Callback<bool>,
  #[prop(optional, into)] id: Option<String>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&[
    "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input dark:data-[state=unchecked]:bg-input/80",
    CONTROL_RING,
    "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
    &class,
  ]);
  view! {
    <button
      type="button"
      role="switch"
      data-slot="switch"
      id=id
      class=class
      aria-checked=move || checked.get().to_string()
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      aria-invalid=invalid_attr(invalid)
      data-state=move || if checked.get() { "checked" } else { "unchecked" }
      data-disabled=move || disabled.get().then_some("")
      disabled=move || disabled.get()
      on:click=move |_| {
        if !disabled.get_untracked() {
          on_change.run(!checked.get_untracked());
        }
      }
    >
      <span
        data-slot="switch-thumb"
        class="pointer-events-none block size-5 rounded-full bg-background shadow-xs ring-0 transition-transform duration-200 data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0 dark:data-[state=unchecked]:bg-foreground dark:data-[state=checked]:bg-primary-foreground"
        data-state=move || if checked.get() { "checked" } else { "unchecked" }
      ></span>
    </button>
  }
}
