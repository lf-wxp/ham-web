use leptos::prelude::*;

/// 快捷键说明行（`label` 与 `keys` 均为中文原文，渲染时按语言翻译）。
///
/// `label` 用 `Signal<String>` 而不是 `String`：说明文案取自 [`crate::i18n::t`]，
/// 传静态字符串的话切换界面语言后这一行不会跟着变。
#[component]
pub fn ShortcutRow(
  #[prop(into)] label: Signal<String>,
  #[prop(into)] keys: String,
) -> impl IntoView {
  view! {
    <div class="flex items-center justify-between">
      <span>{move || label.get()}</span>
      <code class="px-2 py-0.5 rounded border bg-muted">{keys}</code>
    </div>
  }
}
