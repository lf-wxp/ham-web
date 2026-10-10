//! Maidenhead 网格定位速查。

use ham_web_core::grid_system::{GRID_LEVELS, GRID_NOTES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn GridSystemPage() -> impl IntoView {
  set_title("shell.grid-locator");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("radio.maidenhead-grid-locator")
        subtitle=move || t("radio.grid-levels-precision-uses")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.grid-levels")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("radio.digits")}</th>
                  <th class=CELL>{move || t("radio.example-names")}</th>
                  <th class=CELL>{move || t("radio.accuracy")}</th>
                </tr>
              </thead>
              <tbody>
                {GRID_LEVELS
                  .iter()
                  .map(|&(bits, example, precision)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{bits}</td>
                        <td class=format!("{CELL} whitespace-nowrap font-mono")>{example}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{precision}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.notes-2")}</h2>
          <ul class="space-y-2 p-4">
            {GRID_NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*note}</span>
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
