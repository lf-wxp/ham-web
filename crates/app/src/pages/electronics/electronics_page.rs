//! 电子电路基础页面：常见元件、基本电路单元与关键公式。

use ham_web_core::electronics::{CIRCUITS, COMPONENTS, FORMULAS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

use super::component_quiz::ComponentQuiz;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ElectronicsPage() -> impl IntoView {
  set_title("shell.electronics-basics");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.electronics-basics")
        subtitle=move || t("knowledge.common-components-basic-circuits")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.common-electronic-components")}</h2>
          <Table class="min-w-[640px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class="border px-3 py-2 align-top">{move || t("knowledge.component")}</TableHeader>
                <TableHeader class="border px-3 py-2 align-top">{move || t("knowledge.symbol-unit")}</TableHeader>
                <TableHeader class="border px-3 py-2 align-top">{move || t("knowledge.function")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {COMPONENTS
                .iter()
                .map(|&(name, unit, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{name}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-xs text-muted-foreground")>
                        {unit}
                      </TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{desc}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <ComponentQuiz />

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.basic-circuit-units")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CIRCUITS
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
          <h2 class="pxl-section-title">{move || t("knowledge.key-formulas")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FORMULAS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm font-medium text-primary">{t}</span>
                    <span class="font-mono text-sm text-muted-foreground">{d}</span>
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
