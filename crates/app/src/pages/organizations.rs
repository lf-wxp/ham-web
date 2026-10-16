//! 国际组织与分区速查。

use ham_web_core::organizations::{ORGS, ZONES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn OrganizationsPage() -> impl IntoView {
  set_title("shell.organizations-zones");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.organizations-zones")
        subtitle=move || t("knowledge.itu-iaru-arrl-cq")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.main-organisations")}</h2>
          <Table class="min-w-[640px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("tools.abbreviation")}</TableHeader>
                <TableHeader class=CELL>{move || t("common.full-name")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.responsibilities")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {ORGS
                .iter()
                .map(|&(abbr, full, role)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                        {abbr}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{full}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{role}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.zone-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ZONES
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
