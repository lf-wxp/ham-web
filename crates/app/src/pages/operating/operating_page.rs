use ham_web_core::operating::{
  CONTACT_STEPS, GROUNDING_TIPS, LOG_FIELDS, QSL_FIELDS, REPEATER_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::field_list::FieldList;
use super::tip_list::TipList;
use crate::i18n::t;

#[component]
pub fn OperatingPage() -> impl IntoView {
  set_title("shell.operating-practice");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.operating-practice")
        subtitle=move || t("knowledge.contact-procedure-log-qsl")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.standard-contact-procedure")}</h2>
          <FieldList fields=CONTACT_STEPS />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.qso-log-fields")}</h2>
          <FieldList fields=LOG_FIELDS />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.qsl-card-information")}</h2>
          <FieldList fields=QSL_FIELDS />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.repeater-usage-notes")}</h2>
          <TipList tips=REPEATER_TIPS />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("shell.grounding-lightning")}</h2>
          <TipList tips=GROUNDING_TIPS />
        </section>
      </PageContainer>
    </div>
  }
}
