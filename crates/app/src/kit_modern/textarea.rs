//! 现代风多行文本域。
//!
//! 与 [`super::ModernInput`] 同一套约定：受控值是只读的 `Signal`，写回走 `on_input`。
//! 单独一个组件而不是给 `ModernInput` 加分支：`<textarea>` 没有 `type` / `min` / `max`，
//! 挤在一个组件里会让一半的属性对另一半永远是空操作。

use leptos::prelude::*;

#[component]
pub fn ModernTextarea(
  #[prop(into)] value: Signal<String>,
  #[prop(into)] on_input: Callback<String>,
  #[prop(optional, into)] placeholder: Signal<String>,
  /// 没有可见标签时必须传：`<textarea>` 的占位文字在校验失败时报错时会被清掉。
  #[prop(optional, into)]
  aria_label: Signal<String>,
  /// 与 `ui::Field` 的 `r#for` 配对。
  #[prop(optional, into)]
  id: Signal<String>,
  /// 可见行数，默认 3。
  #[prop(optional, into)]
  rows: Signal<String>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let rows = Signal::derive(move || {
    let r = rows.get();
    if r.is_empty() { "3".to_owned() } else { r }
  });

  view! {
    <textarea
      class=format!("md-textarea {class}")
      id=move || {
        let i = id.get();
        (!i.is_empty()).then_some(i)
      }
      rows=move || rows.get()
      aria-label=move || {
        let l = aria_label.get();
        let l = if l.is_empty() { placeholder.get() } else { l };
        (!l.is_empty()).then_some(l)
      }
      placeholder=move || {
        let p = placeholder.get();
        (!p.is_empty()).then_some(p)
      }
      prop:value=move || value.get()
      on:input=move |e| on_input.run(event_target_value(&e))
    ></textarea>
  }
}
