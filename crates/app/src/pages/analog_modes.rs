//! 模拟通信模式速查：CW、SSB、AM、FM 等传统话音与电报模式。

use ham_web_core::analog_modes::{ANALOG_MODES, ANALOG_VS_DIGITAL, SIDEBAND_RULES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn AnalogModesPage() -> impl IntoView {
  set_title("shell.analog-modes");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.analog-modes")
        subtitle=move || t("knowledge.cw-ssb-am-fm")
      />

      <PageContainer>
        <Table class="min-w-[720px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("log.mode")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.emission-type")}</TableHeader>
              <TableHeader class=CELL>{move || t("tools.bandwidth")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.features")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.typical-use")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {ANALOG_MODES
              .iter()
              .map(|m| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>
                      <div class="font-medium">{m.name}</div>
                      <div class="font-mono text-xs text-muted-foreground">{m.abbr}</div>
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-muted-foreground")>{m.emission}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{m.bandwidth}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{m.desc}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{m.pros}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{m.usage}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.usb-lsb-sideband-conventions")}</h2>
          <ul class="space-y-2 p-4">
            {SIDEBAND_RULES
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

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.analogue-vs-digital")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ANALOG_VS_DIGITAL
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
      </PageContainer>
    </div>
  }
}
