//! 频谱管理深化：监测与干扰查处流程、ITU RR 条款结构、共用细则、频率申请与指配。

use ham_web_core::spectrum_admin::{
  ASSIGNMENT_FLOW, MONITORING_STEPS, RR_STRUCTURE, SHARING_RULES,
};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage, ProvenanceNote, StepsSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SpectrumAdminPage() -> impl IntoView {
  set_title("knowledge.spectrum-admin");
  view! {
    <KnowledgePage
      title=move || t("knowledge.spectrum-admin")
      subtitle=move || t("knowledge.spectrum-admin-subtitle")
    >
      <StepsSection title="干扰查处流程" items=MONITORING_STEPS />
      <ConceptsSection title="knowledge.rr-structure" items=RR_STRUCTURE />
      <ConceptsSection title="knowledge.sharing-rules" items=SHARING_RULES />
      <ConceptsSection title="knowledge.assignment-flow" items=ASSIGNMENT_FLOW />
      // 页面分国内流程（《条例》）与 ITU 条款（RR）两块，分别挂各自依据。
      <ProvenanceNote domain="regulations" />
      <ProvenanceNote domain="rr" />
    </KnowledgePage>
  }
}
