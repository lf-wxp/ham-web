use ham_web_core::most_wanted::{WANTED_CONCEPTS, WANTED_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::live_ranking::LiveRanking;
use super::wanted_tracker::WantedTracker;
use crate::i18n::t;

#[component]
pub fn MostWantedPage() -> impl IntoView {
  set_title("shell.dxcc-most-wanted");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.dxcc-most-wanted")
        subtitle=move || t("log.dxcc-most-wanted-list")
      />

      <PageContainer>
        <LiveRanking />

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("log.rarity-concepts")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {WANTED_CONCEPTS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <WantedTracker />

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("log.chasing-strategy")}</h2>
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
      </PageContainer>
    </div>
  }
}
