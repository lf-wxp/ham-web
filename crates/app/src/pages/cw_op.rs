//! CW 操作规范与常用缩语速查。

use ham_web_core::cw_op::{CW_ABBREVIATIONS, CW_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn CwOpPage() -> impl IntoView {
  set_title("shell.cw-operating");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.cw-operating")
        subtitle=move || t("knowledge.common-abbreviations-operating-etiquette")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.common-cw-abbreviations")}</h2>
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
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>{abbr}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{meaning}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {CW_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
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
