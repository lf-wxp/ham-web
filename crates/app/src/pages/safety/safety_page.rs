use ham_web_core::safety::{
  ANTENNA_SAFETY, EXPOSURE_LIMITS, SAFETY_DISTANCE, SAFETY_TIPS, SAR_CONCEPTS,
};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::label_list::LabelList;
use crate::i18n::t;

#[component]
pub fn SafetyPage() -> impl IntoView {
  set_title("shell.rf-safety");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("knowledge.rf-safety-and-electromagnetic")
        subtitle=move || t("knowledge.sar-exposure-limits-safe")
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.basic-concepts")}</h2>
          <LabelList rows=SAR_CONCEPTS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.emf-exposure-limits-gb")}</h2>
          <LabelList rows=EXPOSURE_LIMITS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.safe-distance-reference")}</h2>
          <LabelList rows=SAFETY_DISTANCE />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.safe-operating-notes")}</h2>
          <ul class="space-y-2 p-4">
            {SAFETY_TIPS
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.antenna-installation-safety")}</h2>
          <ul class="space-y-2 p-4">
            {ANTENNA_SAFETY
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
