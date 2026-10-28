//! 新手入门指南速查。

use ham_web_core::beginner::{BEGINNER_STEPS, FIRST_QSO_TIPS, RIG_TYPES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn BeginnerPage() -> impl IntoView {
  set_title("shell.beginner-s-guide-title");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.beginner-s-guide-title")
        subtitle=move || t("knowledge.from-licence-to-first")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.beginner-roadmap")}</h2>
          <div class="space-y-1 p-4">
            {BEGINNER_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2 sm:flex-row sm:items-baseline sm:gap-3">
                    <span class="shrink-0 text-sm text-primary">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.equipment-type")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("radio.type")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.band")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {RIG_TYPES
                .iter()
                .map(|&(name, band, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap")>{move || kt_tracked(name)}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-muted-foreground")>
                        {move || kt_tracked(band)}
                      </TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.first-contact-notes")}</h2>
          <ul class="space-y-2 p-4">
            {FIRST_QSO_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </PageContainer>
    </div>
  }
}
