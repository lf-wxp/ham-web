//! RTTY 与 PSK31 传统数据模式速查。

use ham_web_core::rtty::{RTTY_FREQS, RTTY_MODES, RTTY_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn RttyPage() -> impl IntoView {
  set_title("RTTY / PSK31");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title="RTTY / PSK31".to_string()
        subtitle=move || t("knowledge.radioteletype-phase-shift-keying")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.mode-notes")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("log.mode")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.alias")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {RTTY_MODES
                .iter()
                .map(|&(name, alias, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-primary")>
                        {move || kt_tracked(name)}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(alias)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("shell.common-frequencies")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {RTTY_FREQS
              .iter()
              .map(|&(freq, usage)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="shrink-0 font-mono text-sm text-primary">{move || kt_tracked(freq)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(usage)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {RTTY_TIPS
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
