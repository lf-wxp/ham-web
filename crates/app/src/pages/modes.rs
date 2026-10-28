//! 业余无线电数字语音与数据模式速查。

use ham_web_core::modes::{DIGITAL_MODES, DIGITAL_USAGE};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, SectionCard};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn ModesPage() -> impl IntoView {
  set_title("shell.digital-modes");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=move || t("shell.digital-modes") subtitle=move || t("knowledge.digital-voice-and-data") />
      <PageContainer>
        <Table class="min-w-[720px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("log.mode")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.type")}</TableHeader>
              <TableHeader class=CELL>{move || t("tools.bandwidth")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {DIGITAL_MODES
              .iter()
              .map(|m| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>
                      <div>{move || kt_tracked(m.name)}</div>
                      <div class="font-mono text-xs text-muted-foreground">{move || kt_tracked(m.abbr)}</div>
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(m.kind)}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(m.bandwidth)}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(m.desc)}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <SectionCard title=Signal::derive(move || t("knowledge.choose-a-mode-by"))>
          <dl class="divide-y">
            {DIGITAL_USAGE
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[12rem_1fr]">
                    <dt>{move || kt_tracked(k)}</dt>
                    <dd class="text-sm text-muted-foreground">{move || kt_tracked(v)}</dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </SectionCard>
      </PageContainer>
    </div>
  }
}
