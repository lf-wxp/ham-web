//! 天线分析仪与史密斯圆图速查。

use ham_web_core::antenna_analyzer::{
  ANALYZER_TIPS, ANALYZER_TOOLS, ANALYZER_USES, SMITH_CONCEPTS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaAnalyzerPage() -> impl IntoView {
  set_title("shell.antenna-analyzers-title");
  view! {
    <KnowledgePage title=move || t("shell.antenna-analyzers-title") subtitle=move || t("knowledge.nanovna-impedance-measurement-swr")>
      <TableSection title="测量工具" headers=&["工具", "类型", "说明"] rows=ANALYZER_TOOLS class="min-w-[640px]" />
      <ConceptsSection title="史密斯圆图概念" items=SMITH_CONCEPTS />
      <ConceptsSection title="测量应用" items=ANALYZER_USES />
      <BulletSection title="使用要点" items=ANALYZER_TIPS />
    </KnowledgePage>
  }
}
