use crate::components::common::PageContainer;
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
          view! {
            <PageContainer class="pt-0 pb-10">
              <section class="rounded-xl border bg-card">
                <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("common.related-topics")}</h2>
                <div class="flex flex-wrap gap-2 p-4">
                  {items
                    .iter()
                    .map(|&(href, label)| {
                      view! {
                        <a
                          href=href
                          class="rounded-full border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        >
                          // 文案来自 `crates/core` 的关联表（中文原文），要走反向索引翻一遍 ——
                          // 原样渲染的话英文 / 西语界面上整块「相关主题」都是中文。
                          {move || t(label)}
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
