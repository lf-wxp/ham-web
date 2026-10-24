//! 业余无线电数字语音与数据模式速查。

use ham_web_core::modes::{DIGITAL_MODES, DIGITAL_USAGE};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, SectionCard};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn ModesPage() -> impl IntoView {
  set_title("shell.digital-modes");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=t("shell.digital-modes") subtitle=t("knowledge.digital-voice-and-data") />
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
                      <div>{m.name}</div>
                      <div class="font-mono text-xs text-muted-foreground">{m.abbr}</div>
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{m.kind}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{m.bandwidth}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{m.desc}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <SectionCard title=t("knowledge.choose-a-mode-by")>
          <dl class="divide-y">
            {DIGITAL_USAGE
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[12rem_1fr]">
                    <dt>{k}</dt>
                    <dd class="text-sm text-muted-foreground">{v}</dd>
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
