//! 现代数字模式专题：JS8Call / FreeDV / VARA / ARDOP / M17。

use ham_web_core::modern_digital_modes::{
  MODERN_DIGITAL_CONCEPTS, MODERN_DIGITAL_TABLE, MODERN_DIGITAL_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ModernDigitalModesPage() -> impl IntoView {
  set_title("knowledge.modern-digital-modes");
  view! {
    <KnowledgePage
      title=move || t("knowledge.modern-digital-modes")
      subtitle=move || t("knowledge.modern-digital-modes-subtitle")
    >
      <ConceptsSection title="核心概念" items=MODERN_DIGITAL_CONCEPTS />
      <TableSection
        title="模式对比"
        headers=&["模式", "类型与带宽", "开源与用途"]
        rows=MODERN_DIGITAL_TABLE
        class="min-w-[640px]"
      />
      <BulletSection title="要点" items=MODERN_DIGITAL_TIPS />
    </KnowledgePage>
  }
}
