use ham_web_core::cabrillo::{CABRILLO_CONCEPTS, CABRILLO_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::cabrillo_generator::CabrilloGenerator;
use crate::i18n::t;

#[component]
pub fn CabrilloPage() -> impl IntoView {
  set_title("shell.cabrillo-logs");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.cabrillo-logs")
        subtitle=move || t("log.standard-format-scoring-submission")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("log.format-structure")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CABRILLO_CONCEPTS
              .iter()
              .map(|&(title, desc)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{move || t(title)}</span>
                    <span class="text-sm text-muted-foreground">{move || t(desc)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("log.submission-tips")}</h2>
          <ul class="space-y-2 p-4">
            {CABRILLO_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || t(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <CabrilloGenerator />
      </PageContainer>
    </div>
  }
}
