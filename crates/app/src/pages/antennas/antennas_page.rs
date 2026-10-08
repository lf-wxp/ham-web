use ham_web_core::antennas::{ANTENNA_THEORY, ANTENNAS};
use leptos::prelude::*;

use crate::util::set_title;

use super::antenna_card::AntennaCard;
use crate::components::common::ConceptsSection;
use crate::i18n::t;

#[component]
pub fn AntennasPage() -> impl IntoView {
  set_title("shell.antenna-types");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.antenna-types")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.common-amateur-antennas-diagrams")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl px-4 py-5">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {ANTENNAS.iter().map(|a| view! { <AntennaCard entry=a /> }).collect_view()}
        </div>

        <div class="mt-6">
          <ConceptsSection title="天线理论要点" items=ANTENNA_THEORY />
        </div>
      </div>
    </div>
  }
}
