//! Maidenhead 网格定位速查。

use ham_web_core::grid_system::{GRID_LEVELS, GRID_NOTES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn GridSystemPage() -> impl IntoView {
  set_title("shell.grid-locator");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("radio.maidenhead-grid-locator")
        subtitle=move || t("radio.grid-levels-precision-uses")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.grid-levels")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("radio.digits")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.example-names")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.accuracy")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {GRID_LEVELS
                .iter()
                .map(|&(bits, example, precision)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap")>{move || kt_tracked(bits)}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono")>{move || kt_tracked(example)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(precision)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.notes-heading")}</h2>
          <ul class="space-y-2 p-4">
            {GRID_NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(note)}</span>
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
