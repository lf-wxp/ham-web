//! 开关：替代原生 `<input type="checkbox">` 用于「立即生效」的设置项。
//!
//! 与 [`crate::ui::Checkbox`] 的分工：表单里需要提交 / 校验的勾选用 Checkbox，
//! 设置面板里「切换即生效」的用 Switch（语义上是按钮而非表单项）。

use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;

use super::control::{TextValue, invalid_attr};
use super::led::{Led, LedState, LedTone};

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
  /// 布局类（宽度 / 对齐 / 外边距）。**像素风作用在「灯 + 拨杆 + ON/OFF 丝印」整行上**（行是这一个
  /// 控件的视觉单元），现代风作用在 `<button>` 上；别传只对按钮本体成立的重排类（如 `absolute`）。
  #[prop(optional, into)]
  class: String,
) -> impl IntoView {
  // 像素风是「拨杆 + 旁边的绿灯 + ON / OFF 丝印」（`docs/ui/pixel.md` §3.4）：开 / 关由拨杆位置与
  // `aria-checked` 承担，灯和丝印字是另外两个读法；两者都 `aria-hidden`，读屏只听 `role="switch"`。
  // 现代风换成 `.md-switch`（36×20 药丸、关 = 表面 3 档灰、开 = 品牌蓝、滑块平滑滑动）。
  let modern = crate::kit::is_modern_at_mount();
  let state = move || {
    if checked.get() {
      "checked"
    } else {
      "unchecked"
    }
  };
  let control = view! {
    <button
      type="button"
      role="switch"
      data-slot="switch"
      id=id
      class=if modern { cn(&["md-switch", &class]) } else { "pxl-switch".to_owned() }
      aria-checked=move || checked.get().to_string()
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      aria-invalid=invalid_attr(invalid)
      data-state=state
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
        class=if modern { "md-switch-thumb" } else { "pxl-switch-thumb" }
        data-state=state
      ></span>
    </button>
  };
  if modern {
    return control.into_any();
  }
  let lit = Signal::derive(move || LedState::lit(checked.get() && !disabled.get()));
  view! {
    <span class=cn(&["pxl-switch-row", &class]) data-disabled=move || disabled.get().then_some("")>
      <Led tone=LedTone::Rx state=lit />
      {control}
      <span class="pxl-switch-state" aria-hidden="true" data-state=state>
        {move || if checked.get() { t("common.switch-on") } else { t("common.switch-off") }}
      </span>
    </span>
  }
  .into_any()
}
