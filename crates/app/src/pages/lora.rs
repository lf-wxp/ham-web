//! LoRa / Meshtastic / AREDN 专题：三种数据网络技术的入门与国内合规口径。

use ham_web_core::lora_knowledge::{LORA_COMPLIANCE, LORA_CONCEPTS, LORA_PARAMS, LORA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn LoraPage() -> impl IntoView {
  set_title("knowledge.lora-meshtastic-aredn");
  view! {
    <KnowledgePage
      title=move || t("knowledge.lora-meshtastic-aredn")
      subtitle=move || t("knowledge.lora-meshtastic-aredn-subtitle")
    >
      <ConceptsSection title="核心概念" items=LORA_CONCEPTS />
      <TableSection
        title="LoRa 调制参数"
        headers=&["参数", "取值", "取舍"]
        rows=LORA_PARAMS
        class="min-w-[680px]"
      />
      <TableSection
        title="合规差异"
        headers=&["频段", "法律身份", "执照与功率"]
        rows=LORA_COMPLIANCE
        class="min-w-[760px]"
      />
      <BulletSection title="要点" items=LORA_TIPS />
    </KnowledgePage>
  }
}
