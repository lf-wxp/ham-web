//! 业余无线电历史速查。

use ham_web_core::history::HISTORY_TIMELINE;
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn HistoryPage() -> impl IntoView {
  set_title("shell.amateur-radio-history");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.amateur-radio-history")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.key-figures-milestone-timeline")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.milestone-timeline")}</h2>
          <div class="divide-y">
            {HISTORY_TIMELINE
              .iter()
              .map(|&(year, event)| {
                view! {
                  <div class="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-baseline sm:gap-4">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{year}</span>
                    <span class="text-sm text-muted-foreground">{event}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}
