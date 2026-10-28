//! 电源与电池速查。

use ham_web_core::power::{BATTERIES, POWER_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn PowerPage() -> impl IntoView {
  set_title("shell.power-batteries");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.power-batteries")
        subtitle=move || t("tools.battery-types-dc-power")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("tools.common-battery-types")}</h2>
          <Table class="min-w-[560px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("radio.type")}</TableHeader>
                <TableHeader class=CELL>{move || t("tools.voltage")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {BATTERIES
                .iter()
                .map(|&(name, v, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap")>{move || kt_tracked(name)}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(v)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("tools.power-and-emergency-supply")}</h2>
          <ul class="space-y-2 p-4">
            {POWER_TIPS
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
