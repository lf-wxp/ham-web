//! 业余无线电历史速查。

use ham_web_core::history::HISTORY_TIMELINE;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn HistoryPage() -> impl IntoView {
  set_title("shell.amateur-radio-history");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.amateur-radio-history")
        subtitle=move || t("knowledge.key-figures-milestone-timeline")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.milestone-timeline")}</h2>
          <div class="divide-y">
            {HISTORY_TIMELINE
              .iter()
              .map(|&(year, event)| {
                view! {
                  <div class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-baseline sm:gap-4">
                    <span class="shrink-0 font-mono text-sm text-primary">{move || kt_tracked(year)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(event)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
