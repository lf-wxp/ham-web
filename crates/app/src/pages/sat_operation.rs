//! 业余卫星通联操作指南：FM 中继与线性转发器的完整 QSO 流程。

use ham_web_core::sat_operation::{DOPPLER_TIPS, FM_STEPS, INVERT_RULES, SAT_ETIQUETTE};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, StepsSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SatOperationPage() -> impl IntoView {
  set_title("knowledge.satellite-operating");
  view! {
    <KnowledgePage title=t("knowledge.satellite-operating") subtitle=t("knowledge.the-complete-qso-procedure")>
      <section class="rounded-xl border bg-card p-4 text-sm text-muted-foreground">
        <span>{move || t("knowledge.first-go-to")}</span>
        <a href="/satellites" class="mx-1 font-medium text-primary underline-offset-2 hover:underline">
          {move || t("knowledge.amateur-satellites")}
        </a>
        <span>{move || t("knowledge.to-look-up-pass")}</span>
      </section>
      <StepsSection title="FM 卫星通联步骤" items=FM_STEPS />
      <ConceptsSection title="线性转发器 · 边带倒置" items=INVERT_RULES />
      <BulletSection title="多普勒补偿" items=DOPPLER_TIPS />
      <BulletSection title="通联格式与礼仪" items=SAT_ETIQUETTE />
    </KnowledgePage>
  }
}
