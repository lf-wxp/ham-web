use ham_web_core::phonetic::PHONETIC;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::ui::Stat;
use crate::util::set_title;

use super::phonetic_card::PhoneticCard;
use super::phonetic_listen::PhoneticListen;
use crate::i18n::t;

#[component]
pub fn PhoneticPage() -> impl IntoView {
  set_title("shell.phonetic-alphabet");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.phonetic-alphabet")
        subtitle=move || t("morse.itu-phonetic-alphabet-spell")
      />

      <PageContainer>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-4">
          <Stat label=t("morse.letters") value=PHONETIC.len() />
          <div class="rounded-xl border bg-card p-4 sm:col-span-3">
            <p class="text-sm leading-6 text-muted-foreground">
              {move || t("morse.each-letter-is-represented")}
            </p>
            <p class="mt-2 text-xs text-muted-foreground">{move || t("morse.in-the-pronunciation-guide")}</p>
          </div>
        </div>

        <section>
          <h2 class="mb-3 text-sm font-semibold">{move || t("morse.phonetic-alphabet")}</h2>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
            {PHONETIC.iter().map(|e| view! { <PhoneticCard entry=e /> }).collect_view()}
          </div>
        </section>

        <PhoneticListen />
      </PageContainer>
    </div>
  }
}
