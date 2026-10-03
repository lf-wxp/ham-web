//! 实用天线专题速查：EFHW、磁环小环、接收天线与倒 V。

use ham_web_core::practical_antennas::{
  PRACTICAL_ANTENNAS_CONCEPTS, PRACTICAL_ANTENNAS_TABLE, PRACTICAL_ANTENNAS_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn PracticalAntennasPage() -> impl IntoView {
  set_title(&t("实用天线专题"));
  view! {
    <KnowledgePage title=t("实用天线专题") subtitle=t("EFHW、磁环小环、接收天线与倒 V 的选型要点")>
      <ConceptsSection title="核心概念" items=PRACTICAL_ANTENNAS_CONCEPTS />
      <TableSection title="天线对比" headers=&["天线", "特点", "适用场景"] rows=PRACTICAL_ANTENNAS_TABLE />
      <BulletSection title="制作与使用要点" items=PRACTICAL_ANTENNAS_TIPS />
    </KnowledgePage>
  }
}
