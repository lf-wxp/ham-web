//! 户外便携操作速查：SOTA、POTA 等。

use ham_web_core::portable::{PORTABLE_PROGRAMS, PORTABLE_TIPS};
use leptos::prelude::*;

use super::portable_lookup::PortableLookup;
use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn PortablePage() -> impl IntoView {
  set_title("shell.portable-operation");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.portable-operation")
        subtitle=move || t("radio.sota-summits-pota-parks")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.main-programmes")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("radio.item")}</th>
                  <th class=CELL>{move || t("common.full-name")}</th>
                  <th class=CELL>{move || t("radio.notes")}</th>
                </tr>
              </thead>
              <tbody>
                {PORTABLE_PROGRAMS
                  .iter()
                  .map(|&(name, full, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>{name}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{full}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <PortableLookup />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.portable-notes")}</h2>
          <ul class="space-y-2 p-4">
            {PORTABLE_TIPS
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
