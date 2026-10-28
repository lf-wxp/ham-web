use crate::components::common::PageContainer;
use crate::data;
use crate::i18n::t;
use ham_web_core::related::RELATED;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// 页面底部「相关主题」内链：按当前路由展示关联知识页，把专题串成知识网。
#[component]
pub fn RelatedTopics() -> impl IntoView {
  let location = use_location();
  view! {
    {move || {
      let path = location.pathname.get();
      RELATED
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, items)| {
          // 标签走知识库词典：本组件挂在路由外壳里，页面自己不一定拉了词典。
          data::use_knowledge_i18n();
          view! {
            <PageContainer class="pt-0 pb-10">
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">{move || t("common.related-topics")}</h2>
                <div class="flex flex-wrap gap-2 p-4">
                  {items
                    .iter()
                    .map(|&(href, label)| {
                      view! {
                        <a
                          href=href
                          class="border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        >
                          // 文案来自 `crates/core` 的关联表（中文原文）：先走界面词典的反向索引
                          // （页面标题、分类名），查不到再走知识库词典 —— 两边都不查的话，
                          // 英文 / 西语界面上整块「相关主题」仍是中文。
                          {move || data::core_text_tracked(label)}
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </section>
            </PageContainer>
          }
        })
    }}
  }
}
