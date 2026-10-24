//! 多国题库对照：中国 / 美国 / 英国三套认证体系的结构对照与官方入口。

use ham_web_core::exam_comparison::SYSTEMS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, ProvenanceNote};
use crate::data;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn ExamComparisonPage() -> impl IntoView {
  set_title("knowledge.exam-comparison");
  // 本页自带页头、没套 KnowledgePage 外壳，正文译文得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
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
                <h2 class="flex flex-wrap items-baseline gap-2 pxl-section-title">
                  {move || data::kt_tracked(s.country)}
                  <span class="text-xs font-normal normal-case text-muted-foreground">
                    {move || data::kt_tracked(s.title)}
                  </span>
                </h2>
                <Table class="min-w-[560px] text-sm">
                  <TableHead>
                    <TableRow>
                      <TableHeader class="border px-3 py-2 align-top">
                        {move || t("knowledge.exam-level")}
                      </TableHeader>
                      <TableHeader class="border px-3 py-2 align-top">
                        {move || t("knowledge.exam-privileges")}
                      </TableHeader>
                      <TableHeader class="border px-3 py-2 align-top">
                        {move || t("knowledge.exam-format")}
                      </TableHeader>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {s.levels
                      .iter()
                      .map(|l| {
                        view! {
                          <TableRow>
                            <TableCell class="border px-3 py-2 align-top">
                              {move || data::kt_tracked(l.name)}
                            </TableCell>
                            <TableCell class="border px-3 py-2 align-top text-muted-foreground">
                              {move || data::kt_tracked(l.privileges)}
                            </TableCell>
                            <TableCell class="border px-3 py-2 align-top text-muted-foreground">
                              {move || data::kt_tracked(l.exam)}
                            </TableCell>
                          </TableRow>
                        }
                      })
                      .collect_view()}
                  </TableBody>
                </Table>
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
