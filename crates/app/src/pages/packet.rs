//! Packet Radio 分组无线电与 Winlink 邮件网关速查。

use ham_web_core::packet::{PACKET_APPS, PACKET_CONCEPTS, PACKET_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn PacketPage() -> impl IntoView {
  set_title(&t("Packet Radio 分组无线电"));
  view! {
    <KnowledgePage title=t("Packet Radio 分组无线电") subtitle=t("AX.25 · TNC · Winlink 邮件网关")>
      <ConceptsSection title="核心概念" items=PACKET_CONCEPTS />
      <ConceptsSection title="典型应用" items=PACKET_APPS />
      <BulletSection title="操作要点" items=PACKET_TIPS />
    </KnowledgePage>
  }
}
