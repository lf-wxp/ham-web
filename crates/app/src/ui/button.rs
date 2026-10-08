//! 按钮组件：[`button_class`] 的组件化封装。
//!
//! 原先页面里到处是 `<button class=button_class(Variant::Outline, Size::Sm, "")>`，
//! 交互细节（禁用态、`type`、点击回调）各写一遍。这里收口成组件，样式仍由
//! [`button_class`] 生成，视觉与既有按钮完全一致。

use leptos::html;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use super::control::TextValue;
use super::{Size, Variant, button_class};

/// `<button type>`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonKind {
  #[default]
  Button,
  Submit,
  Reset,
}

impl ButtonKind {
  /// 对应的 `type` 属性值。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Button => "button",
      Self::Submit => "submit",
      Self::Reset => "reset",
    }
  }
}

/// 按钮。
#[component]
pub fn Button(
  #[prop(optional)] variant: Variant,
  #[prop(optional)] size: Size,
  #[prop(optional)] kind: ButtonKind,
  #[prop(optional, into)] disabled: Signal<bool>,
  /// 加载中：显示旋转图标、置 `aria-busy`、并自动禁用，避免重复提交。
  /// 文案保留在按钮里，读屏名字不变，只说「这个控件正忙」。
  #[prop(optional, into)]
  loading: Signal<bool>,
  #[prop(optional, into)] class: String,
  /// 无可见文字时的无障碍名称（图标按钮必填）。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  /// 原生 `title`（鼠标悬停提示）。
  #[prop(optional, into)]
  title: Option<TextValue>,
  #[prop(optional, into)] on_click: Option<Callback<()>>,
  #[prop(optional)] node_ref: NodeRef<html::Button>,
  children: Children,
) -> impl IntoView {
  view! {
    <button
      node_ref=node_ref
      type=kind.as_str()
      data-slot="button"
      class=button_class(variant, size, &class)
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      title=move || title.as_ref().map(TextValue::get)
      aria-busy=move || loading.get().then_some("true")
      disabled=move || disabled.get() || loading.get()
      on:click=move |_| {
        if let Some(cb) = on_click {
          cb.run(());
        }
      }
    >
      {move || {
        loading
          .get()
          .then(|| {
            // 外面包一层 span：`has-[>svg]:px-*` 按「直接子 svg」判断内边距，
            // 直接把 spinner 塞成子 svg 会在加载中把纯文字按钮的左右内边距压窄一档。
            view! {
              <span class="inline-flex shrink-0" aria-hidden="true">
                <Icon kind=IconKind::Loader2 class="size-4 animate-spin" />
              </span>
            }
          })
      }}
      {children()}
    </button>
  }
}
