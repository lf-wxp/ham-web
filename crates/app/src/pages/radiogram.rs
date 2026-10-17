//! NTS Radiogram / ICS-213 专题：应急通信报文的格式速查。

use ham_web_core::radiogram_knowledge::{
  HX_TABLE, ICS213_FIELDS, NTS_PREAMBLE_FIELDS, PRECEDENCE_TABLE, RADIOGRAM_CONCEPTS,
  RADIOGRAM_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RadiogramPage() -> impl IntoView {
  set_title("knowledge.radiogram");
  view! {
    <KnowledgePage
      title=t("knowledge.radiogram")
      subtitle=t("knowledge.radiogram-subtitle")
    >
      <ConceptsSection title="核心概念" items=RADIOGRAM_CONCEPTS />
      <TableSection
        title="NTS 报头字段"
        headers=&["字段", "含义", "是否必填"]
        rows=NTS_PREAMBLE_FIELDS
        class="min-w-[620px]"
      />
      <TableSection
        title="优先级"
        headers=&["缩写", "名称", "说明"]
        rows=PRECEDENCE_TABLE
        class="min-w-[720px]"
      />
      <ConceptsSection title="处理指令 HX" items=HX_TABLE />
      <ConceptsSection title="ICS-213 字段" items=ICS213_FIELDS />
      <BulletSection title="要点" items=RADIOGRAM_TIPS />
    </KnowledgePage>
  }
}
