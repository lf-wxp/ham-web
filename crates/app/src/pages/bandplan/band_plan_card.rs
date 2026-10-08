use ham_web_core::bandplan::BandPlan;
use leptos::prelude::*;

use crate::data;

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
          .map(|seg| {
            view! {
              <li class="flex flex-wrap items-baseline gap-2 text-sm">
                <span class="font-mono tabular-nums text-muted-foreground">{seg.range}</span>
                // 子段文案是知识库正文（中文原文为 key，见 `data/knowledge-i18n/bandplan.json`）：
                // 必须放在 `move ||` 里并订阅加载状态，否则词典到达后这一行不会重算、停在中文。
                <span class="text-muted-foreground">
                  {move || {
                    data::track_knowledge();
                    data::kt(seg.text)
                  }}
                </span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </article>
  }
}
