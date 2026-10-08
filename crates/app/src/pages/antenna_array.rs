//! 天线阵列与相控阵速查。

use ham_web_core::antenna_array::{ARRAY_CONCEPTS, ARRAY_TIPS, ARRAY_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaArrayPage() -> impl IntoView {
  set_title("shell.antenna-arrays-2");
  view! {
    <KnowledgePage title=t("shell.antenna-arrays-2") subtitle=t("knowledge.stacking-gain-beamforming-common")>
      <ConceptsSection title="核心概念" items=ARRAY_CONCEPTS />
      <ConceptsSection title="常见阵列类型" items=ARRAY_TYPES />
      <BulletSection title="设计要点" items=ARRAY_TIPS />
    </KnowledgePage>
  }
}
