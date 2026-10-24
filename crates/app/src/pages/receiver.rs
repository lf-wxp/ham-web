//! 接收机关键指标详解。

use ham_web_core::receiver::{NOISE_BASICS, RECEIVER_METRICS, RECEIVER_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn ReceiverPage() -> impl IntoView {
  set_title("shell.receiver-specs");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.receiver-specs")
        subtitle=move || t("knowledge.sensitivity-selectivity-dynamic-range")
      />

      <PageContainer>
        <Table class="min-w-[640px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("knowledge.metrics")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.meaning")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.deciding-factors")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {RECEIVER_METRICS
              .iter()
              .map(|&(name, meaning, factor)| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>{name}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{meaning}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{factor}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.noise-and-sensitivity-basics")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {NOISE_BASICS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.improving-reception")}</h2>
          <ul class="space-y-2 p-4">
            {RECEIVER_TIPS
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
