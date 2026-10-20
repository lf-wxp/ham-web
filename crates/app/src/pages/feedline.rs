//! 天线匹配与馈线速查。

use ham_web_core::feedline::{FEEDLINES, MATCHING, MISMATCH};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn FeedlinePage() -> impl IntoView {
  set_title("shell.matching-feedlines");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.matching-feedlines")
        subtitle=move || t("knowledge.coax-baluns-atus-swr")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.common-feedlines")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("knowledge.model")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.characteristic-impedance")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {FEEDLINES
                .iter()
                .map(|&(name, z, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{name}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{z}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{desc}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.matching-components-and-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MATCHING
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.mismatch-degree-and-consequences")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MISMATCH
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
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
