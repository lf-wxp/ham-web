//! 多国题库对照：中国 / 美国 / 英国三套认证体系的结构对照与官方入口。

use ham_web_core::exam_comparison::SYSTEMS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, ProvenanceNote};
use crate::data;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ExamComparisonPage() -> impl IntoView {
  set_title("knowledge.exam-comparison");
  // 本页自带页头、没套 KnowledgePage 外壳，正文译文得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("knowledge.exam-comparison")
        subtitle=move || t("knowledge.exam-comparison-subtitle")
      />
      <PageContainer>
        <p class="text-xs text-muted-foreground">
          {move || t("knowledge.exam-comparison-hint")}
        </p>

        {SYSTEMS
          .iter()
          .map(|s| {
            view! {
              <section class="pxl-card border bg-card">
                <header class="flex flex-wrap items-baseline gap-2 border-b px-4 py-3">
                  <span class="text-sm font-semibold">{move || data::kt_tracked(s.country)}</span>
                  <span class="text-xs text-muted-foreground">
                    {move || data::kt_tracked(s.title)}
                  </span>
                </header>
                <div class="overflow-x-auto">
                  <table class="w-full min-w-[560px] border-collapse text-sm">
                    <thead class="bg-muted/60 text-xs">
                      <tr>
                        <th class="border px-3 py-2 text-left align-top">
                          {move || t("knowledge.exam-level")}
                        </th>
                        <th class="border px-3 py-2 text-left align-top">
                          {move || t("knowledge.exam-privileges")}
                        </th>
                        <th class="border px-3 py-2 text-left align-top">
                          {move || t("knowledge.exam-format")}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {s.levels
                        .iter()
                        .map(|l| {
                          view! {
                            <tr>
                              <td class="border px-3 py-2 align-top font-medium">
                                {move || data::kt_tracked(l.name)}
                              </td>
                              <td class="border px-3 py-2 align-top text-muted-foreground">
                                {move || data::kt_tracked(l.privileges)}
                              </td>
                              <td class="border px-3 py-2 align-top text-muted-foreground">
                                {move || data::kt_tracked(l.exam)}
                              </td>
                            </tr>
                          }
                        })
                        .collect_view()}
                    </tbody>
                  </table>
                </div>
                <footer class="flex flex-wrap items-center gap-3 border-t px-4 py-2 text-xs text-muted-foreground">
                  <span>{move || data::kt_tracked(s.authority)}</span>
                  <a
                    href=s.source.1
                    target="_blank"
                    rel="noreferrer"
                    class="text-primary underline-offset-2 hover:underline"
                  >
                    {move || data::kt_tracked(s.source.0)}
                  </a>
                  <span class="ml-auto">{move || data::kt_tracked(s.coverage)}</span>
                </footer>
              </section>
            }
          })
          .collect_view()}

        <ProvenanceNote domain="exam-comparison" />
      </PageContainer>
    </div>
  }
}
