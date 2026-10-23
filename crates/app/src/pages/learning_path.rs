//! 学习路径与认证路线图速查。

use ham_web_core::learning_path::{
  LEARNING_LEVELS, LEARNING_PATH, LEARNING_TIPS, TOPIC_GRAPH, suggested_reviews,
};
use leptos::prelude::*;

use crate::components::common::{
  BulletSection, EmptyState, KnowledgePage, StepsSection, TableSection, TopicChips,
};
use crate::i18n::t;
use crate::study;
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::set_title;

#[component]
pub fn LearningPathPage() -> impl IntoView {
  set_title("knowledge.learning-path");
  // 错题本在本页停留期间不会变，挂载时读一次即可。
  let book = study::load_book();
  let wrong_codes: Vec<&str> = book
    .records
    .iter()
    .filter_map(|r| r.question.p_code())
    .collect();
  let reviews = suggested_reviews(&wrong_codes);

  view! {
    <KnowledgePage title=t("knowledge.learning-path-and-certification") subtitle=t("knowledge.a-step-by-step")>
      <StepsSection title="学习路径" items=LEARNING_PATH />
      <TableSection title="各级别重点" headers=&["级别", "重点", "对应页面"] rows=LEARNING_LEVELS />

      // ── 专题依赖图：按拓扑层渲染，前置专题永远在前 ──────────────
      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">{move || t("knowledge.topic-graph")}</h2>
        <TopicChips nodes=TOPIC_GRAPH.iter().collect() />
        <p class="border-t px-4 py-2 text-xs text-muted-foreground">
          {move || t("knowledge.topic-graph-hint")}
        </p>
      </section>

      // ── 答错回推：按你的错题本回推要先补的专题 ─────────────
      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">{move || t("knowledge.review-before")}</h2>
        {if reviews.is_empty() {
          view! {
            <EmptyState
              plain=true
              title=Signal::derive(move || t("knowledge.review-before-empty"))
              actions=ViewFn::from(move || {
                view! {
                  <ButtonLink href="/practice" variant=Variant::Outline size=Size::Sm>
                    {move || t("exam.practice-all")}
                  </ButtonLink>
                }
              })
            />
          }
            .into_any()
        } else {
          view! {
            <TopicChips nodes=reviews numbered=true />
            <p class="border-t px-4 py-2 text-xs text-muted-foreground">
              {move || t("knowledge.review-before-hint")}
            </p>
          }
            .into_any()
        }}
      </section>

      <BulletSection title="备考建议" items=LEARNING_TIPS />
    </KnowledgePage>
  }
}
