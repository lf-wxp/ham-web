//! 知识库页面外壳：入场动画 + 粘性页头 + 内容容器。

use leptos::prelude::*;

use super::PageContainer;
use super::PageHeader;
use crate::data;
use crate::i18n::{Locale, locale};

/// 知识库页面外壳：入场动画 + 粘性页头 + 内容容器。
///
/// 与直接在页面里写 `<div class="animate-in …">` + `PageHeader` + `PageContainer` 等价，
/// 只是少了三处样板。切到非中文界面时会按需拉取知识库正文译文（见 [`data::kt`]），
/// 没翻译到的条目自动回退中文 —— 因此可以逐模块推进翻译而不影响未完成的模块。
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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title=title subtitle=subtitle />
      <PageContainer>{children()}</PageContainer>
    </div>
  }
}
