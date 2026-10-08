//! 知识库页面外壳：入场动画 + 粘性页头 + 内容容器。

use leptos::prelude::*;

use super::PageContainer;
use super::PageHeader;
use crate::data;

/// 知识库页面外壳：粘性页头 + 内容容器（内容块滚动浮现）。
///
/// 页面级的入场动画由路由过渡统一负责（见 [`crate::motion::RouteTransition`]），
/// 这里不再叠一层 —— 否则「整页淡入」和「内容块逐个浮现」会互相拖慢。
/// 切到非中文界面时会按需拉取知识库正文译文（见 [`data::kt`]），没翻译到的条目
/// 自动回退中文 —— 因此可以逐模块推进翻译而不影响未完成的模块。
#[component]
pub fn KnowledgePage(
  #[prop(into)] title: Signal<String>,
  #[prop(into)] subtitle: Signal<String>,
  children: Children,
) -> impl IntoView {
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div>
      <PageHeader title=title subtitle=subtitle />
      <PageContainer reveal=true>{children()}</PageContainer>
    </div>
  }
}
