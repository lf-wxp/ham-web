//! 知识库页面外壳：入场动画 + 粘性页头 + 内容容器。

use leptos::prelude::*;

use super::PageContainer;
use super::PageHeader;
use crate::data;
use crate::i18n::{Locale, locale};

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
  Effect::new(move |_| {
    let l = locale().get();
    // 按「当前语言的译文是否已就绪」判断，而不是「有没有词典」—— 后者在 zh → en → es
    // 之后会误判为就绪，让西班牙语界面继续显示英文译文。
    if l != Locale::Zh && !data::knowledge_i18n_ready(l.code()) {
      leptos::task::spawn_local(async move {
        data::load_knowledge_i18n(l.code()).await;
      });
    }
  });
  view! {
    <div>
      <PageHeader title=title subtitle=subtitle />
      <PageContainer reveal=true>{children()}</PageContainer>
    </div>
  }
}
