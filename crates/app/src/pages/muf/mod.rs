//! 传播预测与最佳工作频率速查。

mod heatmap_card;
mod voacap_card;

use ham_web_core::muf::{BAND_CHOICE, MUF_CONCEPTS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use crate::i18n::t;
use heatmap_card::HeatmapCard;
use voacap_card::VoacapCard;

#[component]
pub fn MufPage() -> impl IntoView {
  set_title("shell.propagation-forecast");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.propagation-forecast")
        subtitle=move || t("tools.muf-luf-optimum-working")
      />

      <PageContainer>
        <HeatmapCard />

        <VoacapCard />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.key-concepts-2")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MUF_CONCEPTS
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.band-selection-advice")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {BAND_CHOICE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}
