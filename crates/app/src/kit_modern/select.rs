//! 现代风下拉选择：原生 `<select>` + `md-select` 样式。
//!
//! 不用像素的 `ui::SimpleSelect`（它的箭头、描边、字号都是像素语言）；
//! 原生控件保留下拉的系统级可用性（键盘、移动端选择器），样式交给 `.md-select`。

use leptos::prelude::*;

#[component]
pub fn ModernSelect(
  /// 当前值：空串表示「未选择」（显示占位项）。
  value: RwSignal<String>,
  /// 选中后回传值；调用方通常在这里执行动作再把 `value` 清空。
  #[prop(into)]
  on_change: Callback<String>,
  /// `(值, 显示文案)`。
  #[prop(into)]
  options: Signal<Vec<(String, String)>>,
  #[prop(into)] placeholder: Signal<String>,
  #[prop(optional, into)] aria_label: Signal<String>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <select
      class=format!("md-select {class}")
      aria-label=move || {
        let l = aria_label.get();
        if l.is_empty() { placeholder.get() } else { l }
      }
      prop:value=move || value.get()
      on:change=move |e| {
        let v = event_target_value(&e);
        if v.is_empty() {
          return;
        }
        on_change.run(v);
        value.set(String::new());
      }
    >
      <option value="" selected=move || value.get().is_empty()>
        {move || placeholder.get()}
      </option>
      {move || {
        options
          .get()
          .into_iter()
          .map(|(v, label)| view! { <option value=v.clone()>{label}</option> })
          .collect_view()
      }}
    </select>
  }
}
