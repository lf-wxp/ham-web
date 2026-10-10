//! 开关：替代原生 `<input type="checkbox">` 用于「立即生效」的设置项。
//!
//! 与 [`crate::ui::Checkbox`] 的分工：表单里需要提交 / 校验的勾选用 Checkbox，
//! 设置面板里「切换即生效」的用 Switch（语义上是按钮而非表单项）。

use leptos::prelude::*;

use crate::cn::cn;

use super::control::{TextValue, invalid_attr};

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
  // 轨道 / 滑块的像素外观在 `.pxl-switch` / `.pxl-switch-thumb`：关 = 凹陷的暗槽，
  // 开 = 绿底；滑块用 3 帧的 steps() 位移，而不是平滑滑动。开 / 关的信息仍由位置与
  // `aria-checked` 承担，不靠颜色单独传达。
  let class = cn(&["pxl-switch", &class]);
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
        class="pxl-switch-thumb"
        data-state=move || if checked.get() { "checked" } else { "unchecked" }
      ></span>
    </button>
  }
}
