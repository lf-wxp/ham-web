//! SSTV 慢扫描电视：静止图像传输速查。

use ham_web_core::sstv::{SSTV_CONCEPTS, SSTV_FREQS, SSTV_MODES, SSTV_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn SstvPage() -> impl IntoView {
  set_title("shell.sstv");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.sstv")
        subtitle=move || t("knowledge.still-images-within-a")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.key-concepts-heading")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SSTV_CONCEPTS
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
          <h2 class="pxl-section-title">{move || t("shell.common-frequencies")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2 md:grid-cols-3">
            {SSTV_FREQS
              .iter()
              .map(|&(b, f)| {
                view! {
                  <div class="px-3 py-2">
                    <span class="text-sm font-medium">{b}</span>
                    <span class="ml-2 font-mono text-sm text-muted-foreground">{f}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <Table class="min-w-[520px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("log.mode")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.resolution-duration")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {SSTV_MODES
              .iter()
              .map(|&(name, res, desc)| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{name}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{res}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{desc}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {SSTV_TIPS
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
