use ham_web_core::bandplan::BAND_PLANS;
use leptos::prelude::*;

use crate::util::set_title;

use super::band_plan_card::BandPlanCard;

#[component]
pub fn BandPlanPage() -> impl IntoView {
  set_title("波段规划");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"波段规划"</div>
            <div class="text-xs text-muted-foreground">"IARU 三区 · 各波段模式子段（中国大陆口径）"</div>
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
