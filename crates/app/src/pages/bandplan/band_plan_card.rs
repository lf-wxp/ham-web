use ham_web_core::bandplan::BandPlan;
use leptos::prelude::*;

#[component]
pub(super) fn BandPlanCard(plan: &'static BandPlan) -> impl IntoView {
  view! {
    <article class="rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-baseline gap-2">
        <h2 class="font-mono font-semibold text-primary">{plan.band}</h2>
        <span class="text-xs text-muted-foreground">{plan.freq_range}</span>
      </div>
      <ul class="mt-3 space-y-1.5">
        {plan
          .segments
          .iter()
          .map(|&(range, mode)| {
            view! {
              <li class="flex flex-wrap items-baseline gap-2 text-sm">
                <span class="font-mono tabular-nums text-muted-foreground">{range}</span>
                <span class="text-muted-foreground">{mode}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </article>
  }
}
