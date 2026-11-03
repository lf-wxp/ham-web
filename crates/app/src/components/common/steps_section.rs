//! 知识库「步骤 / 流程」列表区块。

use leptos::prelude::*;

use crate::data;
use crate::i18n::t;

/// 步骤 / 流程列表：`(步骤名, 说明)` 逐行排布，步骤名用主色强调。
///
/// 与 [`crate::components::common::ConceptsSection`] 的区别是**纵向单行排列**而非两列网格
/// —— 步骤有先后顺序，两列网格会让阅读顺序错乱。
#[component]
pub fn StepsSection(
  #[prop(into)] title: String,
  items: &'static [(&'static str, &'static str)],
) -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    return view! {
      <section class="md-panel">
        <h2 class="md-panel-title">{move || t(&title)}</h2>
        <ol class="md-steps">
          {items
            .iter()
            .enumerate()
            .map(|(i, (step, desc))| {
              view! {
                <li class="md-step">
                  <span class="md-step-no" aria-hidden="true">{format!("{:02}", i + 1)}</span>
                  <div class="flex flex-col gap-0.5">
                    <span class="md-step-name">{move || { data::track_knowledge(); data::kt(step) }}</span>
                    <span class="md-muted md-sm">{move || { data::track_knowledge(); data::kt(desc) }}</span>
                  </div>
                </li>
              }
            })
            .collect_view()}
        </ol>
      </section>
    }
    .into_any();
  }
  view! {
    <section class=crate::kit::card_class()>
      <h2 class=crate::kit::card_title_class()>{move || t(&title)}</h2>
      <div class="space-y-1 p-4">
        {items
          .iter()
          .map(|(step, desc)| {
            view! {
              <div class="flex flex-col gap-1 px-3 py-2 sm:flex-row sm:items-baseline sm:gap-3">
                <span class="shrink-0 text-sm text-primary">{move || { data::track_knowledge(); data::kt(step) }}</span>
                <span class="text-sm text-muted-foreground">{move || { data::track_knowledge(); data::kt(desc) }}</span>
              </div>
            }
          })
          .collect_view()}
      </div>
    </section>
  }
  .into_any()
}
