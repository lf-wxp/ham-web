use leptos::prelude::*;

/// 快捷键说明行（`label` 与 `keys` 均为中文原文，渲染时按语言翻译）。
#[component]
pub fn ShortcutRow(#[prop(into)] label: String, #[prop(into)] keys: String) -> impl IntoView {
  view! {
    <div class="flex items-center justify-between">
      <span>{label}</span>
      <code class="px-2 py-0.5 rounded border bg-muted">{keys}</code>
    </div>
  }
}
