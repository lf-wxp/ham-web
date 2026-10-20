use leptos::prelude::*;

/// 内容卡片：统一描边与标题栏（`pxl-section-title`），正文由 `children` 提供。
#[component]
pub fn SectionCard(#[prop(into)] title: String, children: Children) -> impl IntoView {
  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{title}</h2>
      {children()}
    </section>
  }
}
