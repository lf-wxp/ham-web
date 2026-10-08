//! 极光通信速查。

use ham_web_core::aurora::{AURORA_BANDS, AURORA_CONCEPTS, AURORA_FORECAST, AURORA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AuroraPage() -> impl IntoView {
  set_title("knowledge.aurora-communication");
  view! {
    <KnowledgePage title=t("knowledge.aurora-communication") subtitle=t("knowledge.polar-region-reflection-on")>
      <ConceptsSection title="核心概念" items=AURORA_CONCEPTS />
      <TableSection title="常用波段" headers=&["波段", "频率", "说明"] rows=AURORA_BANDS />
      <ConceptsSection title="预测与监测" items=AURORA_FORECAST />
      <BulletSection title="操作要点" items=AURORA_TIPS />
    </KnowledgePage>
  }
}
