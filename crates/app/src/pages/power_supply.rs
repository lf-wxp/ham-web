//! 电源供应速查：线性电源、开关电源与稳压供电。

use ham_web_core::power_supply::{SUPPLY_CONCEPTS, SUPPLY_TIPS, SUPPLY_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn PowerSupplyPage() -> impl IntoView {
  set_title(&t("电源供应"));
  view! {
    <KnowledgePage title=t("电源供应") subtitle=t("线性电源 · 开关电源 · 稳压供电")>
      <TableSection title="电源类型" headers=&["类型", "原理", "特点"] rows=SUPPLY_TYPES min_width=640 />
      <ConceptsSection title="稳压与供电概念" items=SUPPLY_CONCEPTS />
      <BulletSection title="选用要点" items=SUPPLY_TIPS />
    </KnowledgePage>
  }
}
