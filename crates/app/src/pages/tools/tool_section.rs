use leptos::prelude::*;

use super::tool_body::ToolBody;
use super::tool_mount::ToolMount;

/// 一个计算器块：卡片 + 标题 + 说明 + 内容。
///
/// 46 个块的外壳完全相同，原先每处都在 `view!` 里手写一遍，Leptos 会为每一处各生成一整套
/// （build / rebuild / mount / unmount）视图代码，合计约 400 KiB。抽成组件后外壳只编译一份；
/// `children` 是类型擦除的 [`Children`]，各个计算器也不会再把外壳按自己的类型单态化一遍。
/// 标题与说明取词条 key（`&'static str`），渲染时再查表，所以会跟随语言切换。
///
/// 外壳立刻渲染、主体按 `ToolMount` 的节奏分批挂上；主体还没挂上时卡片带 `aria-busy`，
/// 读屏知道这里的内容还在路上，测试也靠它等全部挂完再量版式。
#[component]
pub(super) fn ToolSection(
  id: &'static str,
  title: &'static str,
  description: &'static str,
  children: Children,
) -> impl IntoView {
  let ready = expect_context::<ToolMount>().ready(id);
  let busy = move || (!ready.get()).then_some("true");
  if crate::kit::is_modern_at_mount() {
    return view! {
      <section id=id class="scroll-mt-24 md-panel" aria-busy=busy>
        <h2 class="md-panel-title">{move || crate::data::core_text_tracked(title)}</h2>
        <p class="md-muted md-sm">{move || crate::data::core_text_tracked(description)}</p>
        <div class="mt-3"><ToolBody ready=ready>{children()}</ToolBody></div>
      </section>
    }
    .into_any();
  }
  view! {
    <section id=id class=crate::kit::card_class_with("scroll-mt-24") aria-busy=busy>
      <h2 class=crate::kit::card_title_class()>{move || crate::data::core_text_tracked(title)}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">{move || crate::data::core_text_tracked(description)}</p>
      <div class="p-4"><ToolBody ready=ready>{children()}</ToolBody></div>
    </section>
  }
  .into_any()
}
