//! 户外便携操作速查：SOTA、POTA 等。

use ham_web_core::portable::{PORTABLE_PROGRAMS, PORTABLE_TIPS};
use leptos::prelude::*;

use super::portable_lookup::PortableLookup;
use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn PortablePage() -> impl IntoView {
  set_title("shell.portable-operation");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.portable-operation")
        subtitle=move || t("radio.sota-summits-pota-parks")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.main-programmes")}</h2>
          <Table class="min-w-[640px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("radio.item")}</TableHeader>
                <TableHeader class=CELL>{move || t("common.full-name")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {PORTABLE_PROGRAMS
                .iter()
                .map(|&(name, full, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-primary")>{move || kt_tracked(name)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(full)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <PortableLookup />

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.portable-notes")}</h2>
          <ul class="space-y-2 p-4">
            {PORTABLE_TIPS
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
