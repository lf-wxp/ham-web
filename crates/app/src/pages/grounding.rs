//! 接地与防雷速查。

use ham_web_core::grounding::{GROUND_TYPES, GROUNDING_PRACTICE, GROUNDING_TIPS, SURGE_DEVICES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn GroundingPage() -> impl IntoView {
  set_title(&t("接地与防雷"));
  view! {
    <KnowledgePage title=t("接地与防雷") subtitle=t("射频接地 · 防雷接地 · 浪涌保护")>
      <ConceptsSection title="接地类型" items=GROUND_TYPES />
      <ConceptsSection title="接地做法" items=GROUNDING_PRACTICE />
      <ConceptsSection title="防雷器件" items=SURGE_DEVICES />
      <BulletSection title="防雷操作要点" items=GROUNDING_TIPS />
    </KnowledgePage>
  }
}
