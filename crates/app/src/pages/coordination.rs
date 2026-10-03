//! 频率协调速查：IARU 分区差异、协调层级、申请与干扰处理流程。

use ham_web_core::coordination::{
  COORDINATION_CONCEPTS, COORDINATION_STEPS, COORDINATION_TIERS, COORDINATION_TIPS,
  IARU_BAND_DIFFS, INTERFERENCE_STEPS,
};
use leptos::prelude::*;

use crate::components::common::{
  BulletSection, ConceptsSection, KnowledgePage, StepsSection, TableSection,
};
use crate::i18n::t;
use crate::util::set_title;

/// IARU 区间差异表的表头。
const IARU_HEADERS: &[&str] = &["波段", "IARU 一区", "IARU 二区 / 三区"];

#[component]
pub fn CoordinationPage() -> impl IntoView {
  set_title(&t("频率协调"));
  view! {
    <KnowledgePage title=t("频率协调") subtitle=t("IARU 分区 · 协调层级 · 申请与干扰处理")>
      <ConceptsSection title="核心概念" items=COORDINATION_CONCEPTS />
      <StepsSection title="协调层级（自顶向下）" items=COORDINATION_TIERS />
      <TableSection
        title="IARU 三区波段差异"
        headers=IARU_HEADERS
        rows=IARU_BAND_DIFFS
        min_width=720
      />
      <StepsSection title="频率协调申请流程" items=COORDINATION_STEPS />
      <StepsSection title="受干扰处理流程" items=INTERFERENCE_STEPS />
      <BulletSection title="要点与提醒" items=COORDINATION_TIPS />
    </KnowledgePage>
  }
}
