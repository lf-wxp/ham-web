//! VNA 矢量网络分析仪实操速查。

use ham_web_core::vna::{VNA_CONCEPTS, VNA_STEPS, VNA_USES};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn VnaPage() -> impl IntoView {
  set_title("knowledge.vna-vector-network-analyzer");
  view! {
    <KnowledgePage title=t("knowledge.vna-vector-network-analyzer") subtitle=t("knowledge.calibration-methods-and-antenna")>
      <ConceptsSection title="核心概念" items=VNA_CONCEPTS />
      <StepsSection title="测量流程" items=VNA_STEPS />
      <TableSection title="常见测量对象" headers=&["对象", "主要看", "意义"] rows=VNA_USES />
    </KnowledgePage>
  }
}
