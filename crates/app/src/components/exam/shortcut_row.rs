use leptos::prelude::*;

/// 快捷键说明行。
#[component]
pub fn ShortcutRow(label: &'static str, keys: &'static str) -> impl IntoView {
  view! {
    <div class="flex items-center justify-between">
      <span>{label}</span>
      <code class="px-2 py-0.5 rounded border bg-muted">{keys}</code>
    </div>
  }
}
