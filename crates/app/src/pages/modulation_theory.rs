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
    <KnowledgePage title=move || t("knowledge.modulation-theory") subtitle=move || t("knowledge.how-information-is-loaded")>
      <ConceptsSection title="核心概念" items=MOD_CONCEPTS />
      <TableSection
        title="模拟调制方式对比"
        headers=&["方式", "关键参数", "说明"]
        rows=MOD_SCHEMES
        class="min-w-[760px]"
      />
      <TableSection
        title="带宽与调制指数"
        headers=&["模式", "带宽 / 条件", "说明"]
        rows=MOD_BANDWIDTH
        class="min-w-[760px]"
      />
      <ConceptsSection title="FM 的四个关键效应" items=FM_EFFECTS />
      <TableSection
        title="解调（检波）方式"
        headers=&["方式", "适用", "原理"]
        rows=DEMOD_METHODS
        class="min-w-[760px]"
      />
      <BulletSection title="要点" items=MOD_TIPS />
    </KnowledgePage>
  }
}
