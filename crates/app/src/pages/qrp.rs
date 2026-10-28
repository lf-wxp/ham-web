//! QRP 低功率操作速查。

use ham_web_core::qrp::{QRP_CONCEPTS, QRP_RIGS, QRP_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn QrpPage() -> impl IntoView {
  set_title("shell.qrp-title");
  view! {
    <KnowledgePage title=move || t("shell.qrp-title") subtitle=move || t("knowledge.the-qrp-mindset-operating")>
      <ConceptsSection title="核心概念" items=QRP_CONCEPTS />
      <ConceptsSection title="设备建议" items=QRP_RIGS />
      <BulletSection title="操作要点" items=QRP_TIPS />
    </KnowledgePage>
  }
}
