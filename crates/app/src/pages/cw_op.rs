//! CW 操作规范与常用缩语速查。

use ham_web_core::cw_op::{CW_ABBREVIATIONS, CW_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::{self, kt_tracked};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn CwOpPage() -> impl IntoView {
  set_title("shell.cw-operating");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.cw-operating")
        subtitle=move || t("knowledge.common-abbreviations-operating-etiquette")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.common-cw-abbreviations")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("knowledge.abbreviations")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.meaning")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {CW_ABBREVIATIONS
                .iter()
                .map(|&(abbr, meaning)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-primary")>{move || kt_tracked(abbr)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(meaning)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {CW_TIPS
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
