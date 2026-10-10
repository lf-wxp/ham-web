//! APRS 自动位置报告系统速查。

use ham_web_core::aprs::{APRS_CONCEPTS, APRS_FREQS, APRS_USES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AprsPage() -> impl IntoView {
  set_title("APRS");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("knowledge.aprs-automatic-position-reporting")
        subtitle=move || t("knowledge.position-tracking-weather-stations")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.key-concepts-2")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {APRS_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("shell.common-frequencies")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {APRS_FREQS
              .iter()
              .map(|&(freq, usage)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{freq}</span>
                    <span class="text-sm text-muted-foreground">{usage}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.main-uses")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {APRS_USES
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
      </PageContainer>
    </div>
  }
}
