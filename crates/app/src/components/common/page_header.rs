use leptos::prelude::*;

use crate::cn::cn;

/// 页面头部：粘性标题栏（标题 + 副标题），右侧可放操作区（如搜索框、筛选器）。
///
/// `title` / `subtitle` 为 [`Signal<String>`]，既接受静态 [`String`]（`From<String>`），
/// 也接受响应式信号（如 `Signal::derive(move || t("…"))`），切语言时即时刷新。
/// `class` 可覆盖粘性偏移（如 `top-16`，用于页面上方还有其它粘性栏的页面）。
#[component]
pub fn PageHeader(
  #[prop(into)] title: Signal<String>,
  #[prop(into)] subtitle: Signal<String>,
  #[prop(optional, into)] class: String,
  #[prop(optional, into)] actions: Option<ViewFn>,
) -> impl IntoView {
  view! {
    <header class=cn(&["sticky top-0 z-20 border-b bg-background/90 backdrop-blur", &class])>
      <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
        <div class="mr-auto">
          <h1 class="text-base font-semibold leading-tight">{move || title.get()}</h1>
          <div class="text-xs text-muted-foreground">{move || subtitle.get()}</div>
        </div>
        {actions.map(|a| a.run())}
      </div>
    </header>
  }
}
