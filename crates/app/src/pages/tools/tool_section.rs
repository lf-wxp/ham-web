use leptos::prelude::*;

use crate::i18n::t;

/// 一个计算器块：卡片 + 标题 + 说明 + 内容。
///
/// 46 个块的外壳完全相同，原先每处都在 `view!` 里手写一遍，Leptos 会为每一处各生成一整套
/// （build / rebuild / mount / unmount）视图代码，合计约 400 KiB。抽成组件后外壳只编译一份；
/// `children` 是类型擦除的 [`Children`]，各个计算器也不会再把外壳按自己的类型单态化一遍。
/// 标题与说明取词条 key（`&'static str`），渲染时再查表，所以会跟随语言切换。
#[component]
pub(super) fn ToolSection(
  id: &'static str,
  title: &'static str,
  description: &'static str,
  children: Children,
) -> impl IntoView {
  view! {
    <section id=id class="scroll-mt-24 pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t(title)}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t(description)}</p>
      <div class="p-4">{children()}</div>
    </section>
  }
}
