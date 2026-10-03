//! 接收环境与噪声速查。

use ham_web_core::noise::{NOISE_CONCEPTS, NOISE_TIPS, NOISE_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NoisePage() -> impl IntoView {
  set_title(&t("接收环境与噪声"));
  view! {
    <KnowledgePage title=t("接收环境与噪声") subtitle=t("QRN / QRM 与底噪的识别和应对")>
      <ConceptsSection title="核心概念" items=NOISE_CONCEPTS />
      <TableSection title="噪声类型" headers=&["类型", "来源", "影响"] rows=NOISE_TYPES />
      <BulletSection title="降低噪声的要点" items=NOISE_TIPS />
    </KnowledgePage>
  }
}
