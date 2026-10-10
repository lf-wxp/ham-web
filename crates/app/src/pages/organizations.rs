//! 国际组织与分区速查。

use ham_web_core::organizations::{ORGS, ZONES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

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
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.main-organisations")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("tools.abbreviation")}</th>
                  <th class=CELL>{move || t("common.full-name")}</th>
                  <th class=CELL>{move || t("knowledge.responsibilities")}</th>
                </tr>
              </thead>
              <tbody>
                {ORGS
                  .iter()
                  .map(|&(abbr, full, role)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {abbr}
                        </td>
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{full}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{role}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.zone-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ZONES
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
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
