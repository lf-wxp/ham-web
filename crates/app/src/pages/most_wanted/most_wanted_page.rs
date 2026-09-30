use ham_web_core::most_wanted::{WANTED_CONCEPTS, WANTED_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

use super::live_ranking::LiveRanking;
use super::wanted_tracker::WantedTracker;

#[component]
pub fn MostWantedPage() -> impl IntoView {
  set_title("DXCC 稀有度榜单");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-16 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"DXCC 稀有度榜单"</div>
            <div class="text-xs text-muted-foreground">"DXCC 世纪俱乐部最稀有榜 · 稀有实体追台"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <LiveRanking />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"稀有度概念"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {WANTED_CONCEPTS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <WantedTracker />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"追台策略"</h2>
          <ul class="space-y-2 p-4">
            {WANTED_TIPS
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
