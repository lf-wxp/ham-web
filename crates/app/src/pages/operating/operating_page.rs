use ham_web_core::operating::{
  CONTACT_STEPS, GROUNDING_TIPS, LOG_FIELDS, QSL_FIELDS, REPEATER_TIPS,
};
use leptos::prelude::*;

use crate::util::set_title;

use super::field_list::FieldList;
use super::tip_list::TipList;
use crate::i18n::t;

#[component]
pub fn OperatingPage() -> impl IntoView {
  set_title("shell.operating-practice");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.operating-practice")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.contact-procedure-log-qsl")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.standard-contact-procedure")}</h2>
          <FieldList fields=CONTACT_STEPS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.qso-log-fields")}</h2>
          <FieldList fields=LOG_FIELDS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.qsl-card-information")}</h2>
          <FieldList fields=QSL_FIELDS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.repeater-usage-notes")}</h2>
          <TipList tips=REPEATER_TIPS />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("shell.grounding-lightning")}</h2>
          <TipList tips=GROUNDING_TIPS />
        </section>
      </div>
    </div>
  }
}
