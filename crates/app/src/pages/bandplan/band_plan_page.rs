use ham_web_core::bandplan::BAND_PLANS;
use leptos::prelude::*;

use crate::util::set_title;

use super::band_plan_card::BandPlanCard;
use crate::i18n::t;

#[component]
pub fn BandPlanPage() -> impl IntoView {
  set_title("shell.band-plan");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.band-plan")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.iaru-region-3-band")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl px-4 py-5">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {BAND_PLANS.iter().map(|b| view! { <BandPlanCard plan=b /> }).collect_view()}
        </div>
      </div>
    </div>
  }
}
