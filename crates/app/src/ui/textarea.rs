//! 多行文本域：替代原生 `<textarea>`（笔记编辑、ADIF 粘贴框等）。

use leptos::html;
use leptos::prelude::*;

use crate::cn::cn;

use super::control::{CONTROL_BASE, TextValue, invalid_attr};

/// 多行文本域。
#[component]
pub fn Textarea(
  /// 当前值（受控）。
  #[prop(into)]
  value: Signal<String>,
  /// 每次输入后的新值。
  on_change: Callback<String>,
  /// 初始可见行数（同时作为最小高度）。
  #[prop(optional, default = 3)]
  rows: u32,
  /// 随内容自动增高（去掉内部滚动条，用于笔记一类不定长输入）。
  #[prop(optional)]
  auto_resize: bool,
  #[prop(optional, into)] placeholder: Option<TextValue>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] name: Option<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] readonly: Signal<bool>,
  /// 错误态。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  #[prop(optional)] autofocus: bool,
  #[prop(optional, into)] class: String,
  #[prop(optional)] node_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
  // 没有前缀 / 后缀插槽，不需要外包裹层；`py-2` 覆盖基线的 `py-1`，
  // `min-h` 只在自动增高时接管（否则交给 `rows`）。
  let class = cn(&[
    CONTROL_BASE,
    "resize-y py-2 leading-relaxed",
    if auto_resize {
      "resize-none overflow-hidden"
    } else {
      ""
    },
    &class,
  ]);

  if auto_resize {
    let resize = move |el: web_sys::HtmlTextAreaElement| {
      let style = web_sys::HtmlElement::style(&el);
      let _ = style.set_property("height", "auto");
      let h = el.scroll_height();
      let _ = style.set_property("height", &format!("{h}px"));
    };
    let sync = node_ref;
    Effect::new(move |_| {
      let _ = value.get();
      if let Some(el) = sync.get() {
        resize(el);
      }
    });
    // 首次也要对齐一次：effect 可能早于元素挂载执行，那时 `scroll_height` 还是 0。
    node_ref.on_load(resize);
  }

  view! {
    <textarea
      node_ref=node_ref
      data-slot="textarea"
      id=id
      name=name
      rows=rows
      placeholder=move || placeholder.as_ref().map(TextValue::get)
      aria-label=move || aria_label.as_ref().map(TextValue::get)
      aria-invalid=invalid_attr(invalid)
      autofocus=autofocus
      disabled=move || disabled.get()
      readonly=move || readonly.get()
      class=class
      prop:value=move || value.get()
      on:input=move |e| on_change.run(event_target_value(&e))
    ></textarea>
  }
}
