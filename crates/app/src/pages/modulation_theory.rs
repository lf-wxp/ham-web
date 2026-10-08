//! 调制理论速查页。

use ham_web_core::modulation_theory::{
  DEMOD_METHODS, FM_EFFECTS, MOD_BANDWIDTH, MOD_CONCEPTS, MOD_SCHEMES, MOD_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ModulationTheoryPage() -> impl IntoView {
  set_title("knowledge.modulation-theory");
  view! {
    <KnowledgePage title=t("knowledge.modulation-theory") subtitle=t("knowledge.how-information-is-loaded")>
      <ConceptsSection title="核心概念" items=MOD_CONCEPTS />
      <TableSection
        title="模拟调制方式对比"
        headers=&["方式", "关键参数", "说明"]
        rows=MOD_SCHEMES
        min_width=760
      />
      <TableSection
        title="带宽与调制指数"
        headers=&["模式", "带宽 / 条件", "说明"]
        rows=MOD_BANDWIDTH
        min_width=760
      />
      <ConceptsSection title="FM 的四个关键效应" items=FM_EFFECTS />
      <TableSection
        title="解调（检波）方式"
        headers=&["方式", "适用", "原理"]
        rows=DEMOD_METHODS
        min_width=760
      />
      <BulletSection title="要点" items=MOD_TIPS />
    </KnowledgePage>
  }
}
