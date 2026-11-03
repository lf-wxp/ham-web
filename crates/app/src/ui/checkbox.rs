//! 复选框（对应 Radix Checkbox）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::control::{TextValue, invalid_attr};

#[component]
pub fn Checkbox(
  #[prop(into)] checked: Signal<bool>,
  on_change: Callback<bool>,
  #[prop(optional, into)] id: Option<String>,
  /// 无可见标签时的无障碍名称（表格里逐行的勾选必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  /// 校验未通过（如「必须勾选同意」）：描边变红，读屏念「无效」。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  // 现代风：`md-check`（16px 圆角方块，选中橙底 + 白色对勾）。
  let modern = crate::kit::is_modern_at_mount();
  let class = if modern {
    cn(&["md-check peer", &class])
  } else {
    cn(&["pxl-check peer", &class])
  };
  view! {
    <button
      type="button"
      role="checkbox"
      data-slot="checkbox"
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
      {move || {
        checked
          .get()
          .then(|| {
            view! {
              <span
                data-slot="checkbox-indicator"
                data-state="checked"
                class=if modern {
                  "md-check-mark"
                } else {
                  "motion-check flex items-center justify-center text-current"
                }
                style="pointer-events: none;"
              >
                <Icon
                  kind=IconKind::Check
                  class=if modern { "md-check-mark-icon" } else { "size-6" }
                />
              </span>
            }
          })
      }}
    </button>
  }
}
