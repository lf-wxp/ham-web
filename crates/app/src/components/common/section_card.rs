use leptos::prelude::*;

/// 内容卡片：统一圆角边框与标题栏，正文由 `children` 提供。
#[component]
pub fn SectionCard(#[prop(into)] title: String, children: Children) -> impl IntoView {
  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{title}</h2>
      {children()}
    </section>
  }
}
