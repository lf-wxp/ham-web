//! SDR 硬件与 GNU Radio 速查。

use ham_web_core::gnuradio::{GNU_RADIO_CONCEPTS, GNU_RADIO_TIPS, SDR_HARDWARE};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn GnuradioPage() -> impl IntoView {
  set_title("shell.sdr-gnu-radio");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.sdr-gnu-radio")
        subtitle=move || t("knowledge.choosing-a-receiver-sdr")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.hardware-selection")}</h2>
          <Table class="min-w-[520px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("knowledge.hardware")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.type")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {SDR_HARDWARE
                .iter()
                .map(|&(name, kind, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-primary")>
                        {name}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{kind}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{desc}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.gnu-radio-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {GNU_RADIO_CONCEPTS
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
          <h2 class="pxl-section-title">{move || t("knowledge.getting-started")}</h2>
          <ul class="space-y-2 p-4">
            {GNU_RADIO_TIPS
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
