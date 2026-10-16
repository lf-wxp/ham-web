//! 数字通信原理速查页。

use ham_web_core::digital_comms::{
  CHANNEL_CODING, DIGITAL_CONCEPTS, DIGITAL_FORMULAS, DIGITAL_SCHEMES, DIGITAL_STEPS, DIGITAL_TIPS,
  WEAK_SIGNAL_PROTOCOLS,
};
use leptos::prelude::*;

use crate::components::common::{
  BulletSection, ConceptsSection, KnowledgePage, StepsSection, TableSection,
};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DigitalCommsPage() -> impl IntoView {
  set_title("knowledge.digital-communications");
  view! {
    <KnowledgePage title=t("knowledge.digital-communications") subtitle=t("knowledge.how-bits-travel-reliably")>
      <ConceptsSection title="核心概念" items=DIGITAL_CONCEPTS />
      <TableSection
        title="数字调制方式"
        headers=&["方式", "每符号比特", "特点"]
        rows=DIGITAL_SCHEMES
        class="min-w-[720px]"
      />
      <TableSection
        title="关键公式"
        headers=&["名称", "公式", "含义"]
        rows=DIGITAL_FORMULAS
        class="min-w-[720px]"
      />
      <TableSection
        title="信道编码与差错控制"
        headers=&["编码", "原理", "用途 / 特点"]
        rows=CHANNEL_CODING
        class="min-w-[720px]"
      />
      <TableSection
        title="弱信号协议机理"
        headers=&["协议", "调制与周期", "机理"]
        rows=WEAK_SIGNAL_PROTOCOLS
        class="min-w-[560px]"
      />
      <StepsSection title="数字通信链路" items=DIGITAL_STEPS />
      <BulletSection title="要点" items=DIGITAL_TIPS />
    </KnowledgePage>
  }
}
