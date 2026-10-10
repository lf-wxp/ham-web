//! 滤波器与双工器速查。

use ham_web_core::filters::{FILTER_TIPS, FILTER_TYPES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn FiltersPage() -> impl IntoView {
  set_title("shell.filters-duplexers");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.filters-duplexers")
        subtitle=move || t("tools.low-pass-band-pass")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.component-type")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("tools.component")}</th>
                  <th class=CELL>{move || t("tools.abbreviation")}</th>
                  <th class=CELL>{move || t("radio.notes")}</th>
                </tr>
              </thead>
              <tbody>
                {FILTER_TYPES
                  .iter()
                  .map(|&(name, abbr, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {abbr}
                        </td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.selection-notes")}</h2>
          <ul class="space-y-2 p-4">
            {FILTER_TIPS
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
