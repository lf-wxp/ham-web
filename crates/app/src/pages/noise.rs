//! 接收环境与噪声速查。

use ham_web_core::noise::{NOISE_CONCEPTS, NOISE_TIPS, NOISE_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NoisePage() -> impl IntoView {
  set_title("knowledge.reception-environment-and-noise");
  view! {
    <KnowledgePage title=t("knowledge.reception-environment-and-noise") subtitle=t("knowledge.identifying-and-dealing-with")>
      <ConceptsSection title="核心概念" items=NOISE_CONCEPTS />
      <TableSection title="噪声类型" headers=&["类型", "来源", "影响"] rows=NOISE_TYPES />
      <BulletSection title="降低噪声的要点" items=NOISE_TIPS />
    </KnowledgePage>
  }
}
