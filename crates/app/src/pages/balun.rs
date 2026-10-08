//! 巴伦与不平衡变压器速查。

use ham_web_core::balun::{BALUN_CONCEPTS, BALUN_RATIOS, BALUN_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn BalunPage() -> impl IntoView {
  set_title("knowledge.baluns-and-ununs");
  view! {
    <KnowledgePage title=t("knowledge.baluns-and-ununs") subtitle=t("knowledge.balanced-to-unbalanced-impedance")>
      <ConceptsSection title="核心概念" items=BALUN_CONCEPTS />
      <TableSection title="阻抗比与用途" headers=&["阻抗比", "典型用途", "说明"] rows=BALUN_RATIOS min_width=640 />
      <BulletSection title="绕制与选型要点" items=BALUN_TIPS />
    </KnowledgePage>
  }
}
