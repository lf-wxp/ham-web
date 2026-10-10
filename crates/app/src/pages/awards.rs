//! DX 奖状体系速查。

use ham_web_core::awards::AWARDS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AwardsPage() -> impl IntoView {
  set_title("shell.dx-awards-2");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.dx-awards-2")
        subtitle=move || t("log.dxcc-waz-all-cq")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("log.major-international-awards")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[720px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("log.abbreviation")}</th>
                  <th class=CELL>{move || t("common.full-name")}</th>
                  <th class=CELL>{move || t("log.organisation")}</th>
                  <th class=CELL>{move || t("log.basic-rules")}</th>
                </tr>
              </thead>
              <tbody>
                {AWARDS
                  .iter()
                  .map(|&(abbr, full, org, rule)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {abbr}
                        </td>
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{full}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{org}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{rule}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
