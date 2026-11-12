//! 天线补充专题：垂直天线与地网、陷阱偶极子、对数周期天线、dBi 与 dBd 的增益口径。

use ham_web_core::antenna_topics::{
  GAIN_CONCEPTS, GAIN_TIPS, LPDA_CONCEPTS, LPDA_TIPS, TRAP_CONCEPTS, TRAP_TIPS, VERTICAL_CONCEPTS,
  VERTICAL_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaTopicsPage() -> impl IntoView {
  set_title("shell.antenna-topics");
  view! {
    <KnowledgePage
      title=move || t("shell.antenna-topics")
      subtitle=move || t("knowledge.antenna-topics-subtitle")
    >
      <ConceptsSection title="knowledge.antenna-topics-vertical" items=VERTICAL_CONCEPTS />
      <BulletSection title="knowledge.antenna-topics-vertical-tips" items=VERTICAL_TIPS />
      <ConceptsSection title="knowledge.antenna-topics-trap" items=TRAP_CONCEPTS />
      <BulletSection title="knowledge.antenna-topics-trap-tips" items=TRAP_TIPS />
      <ConceptsSection title="knowledge.antenna-topics-lpda" items=LPDA_CONCEPTS />
      <BulletSection title="knowledge.antenna-topics-lpda-tips" items=LPDA_TIPS />
      <ConceptsSection title="knowledge.antenna-topics-gain" items=GAIN_CONCEPTS />
      <BulletSection title="knowledge.antenna-topics-gain-tips" items=GAIN_TIPS />
    </KnowledgePage>
  }
}
