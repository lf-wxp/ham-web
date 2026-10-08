//! 链接版按钮：`<a href>` + 与 [`Button`](super::Button) 同一套类名。
//!
//! 为什么不给 `Button` 加一个 `href` 分支：`<a>` 与 `<button>` 在语义与浏览器行为上
//! 完全不同 —— 链接能被中键新开、右键复制地址、被读屏念成「链接」，而按钮只会触发动作。
//! **要跳转就用链接，要执行动作才用按钮**；视觉一致靠同一份 [`button_class`]，不必把两者
//! 塞进同一个组件（塞进去反而会让「这是个链接」这件事在调用点看不出来）。
//!
//! 典型用法见 `src/ui/README.md`。

use leptos::prelude::*;

use super::control::TextValue;
use super::{Size, Variant, button_class};

/// 链接版按钮（导航 / 跳转用）。
#[component]
pub fn ButtonLink(
  /// 目标地址。与 `aria_label` 同一套 [`TextValue`]：静态字符串、`String`、
  /// `Signal<String>`（`Signal::derive(move || …)` / `RwSignal`）都可以 ——
  /// 因为地址有时依赖页面状态（题库版本、筛选条件）。
  #[prop(into)]
  href: TextValue,
  #[prop(optional)] variant: Variant,
  #[prop(optional)] size: Size,
  #[prop(optional, into)] class: String,
  /// 无可见文字时的无障碍名称（图标链接必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 原生 `title`（鼠标悬停提示）。
  #[prop(optional, into)]
  title: Option<TextValue>,
  /// 外部链接用 `target="_blank"`，并同时给 `rel="noopener noreferrer"`。
  #[prop(optional, into)]
  target: Option<String>,
  #[prop(optional, into)] rel: Option<String>,
  /// 点击回调。拿到的是 [`web_sys::MouseEvent`]，因此「条件不满足时
  /// `prevent_default()` 拦住这次跳转」这种守卫可以直接写在调用点。
  #[prop(optional, into)]
  on_click: Option<Callback<web_sys::MouseEvent>>,
  children: Children,
) -> impl IntoView {
  view! {
    <a
      href=move || href.get()
      data-slot="button"
      class=button_class(variant, size, &class)
      target=target
      rel=rel
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      title=move || title.as_ref().map(TextValue::get)
      on:click=move |e| {
        if let Some(cb) = on_click {
          cb.run(e);
        }
      }
    >
      {children()}
    </a>
  }
}
