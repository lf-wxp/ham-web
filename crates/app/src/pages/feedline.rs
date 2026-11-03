//! 天线匹配与馈线速查。

use ham_web_core::feedline::{FEEDLINES, MATCHING, MISMATCH};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn FeedlinePage() -> impl IntoView {
  set_title("shell.matching-feedlines");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.matching-feedlines")
        subtitle=move || t("knowledge.coax-baluns-atus-swr")
      />

      <PageContainer>
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("knowledge.common-feedlines")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("knowledge.model")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("knowledge.characteristic-impedance")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {FEEDLINES
                .iter()
                .map(|&(name, z, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>{move || kt_tracked(name)}</TableCell>
                      <TableCell class=crate::kit::table_cell_class("whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(z)}</TableCell>
                      <TableCell class=crate::kit::table_cell_class("text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("knowledge.matching-components-and-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MATCHING
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("knowledge.mismatch-degree-and-consequences")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MISMATCH
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="shrink-0 font-mono text-sm text-primary">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
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
