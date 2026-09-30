//! 天线调试实操速查。

use ham_web_core::antenna_tuning::{TUNING_STEPS, TUNING_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

#[component]
pub fn AntennaTuningPage() -> impl IntoView {
  set_title("天线调试");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"天线调试"</div>
            <div class="text-xs text-muted-foreground">"天线分析仪 · 修剪流程"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"调试步骤"</h2>
          <div class="space-y-1 p-4">
            {TUNING_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2 sm:flex-row sm:items-baseline sm:gap-3">
                    <span class="shrink-0 text-sm font-semibold text-primary">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"调试要点"</h2>
          <ul class="space-y-2 p-4">
            {TUNING_TIPS
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
      </div>
    </div>
  }
}
