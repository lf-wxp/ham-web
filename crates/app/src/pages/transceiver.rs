//! 收发信机原理与选购速查。

use ham_web_core::transceiver::{BUYING_TIPS, RECEIVER_METRICS, TRANSCEIVER_CONCEPTS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn TransceiverPage() -> impl IntoView {
  set_title("shell.transceivers");
  view! {
    <KnowledgePage title=t("shell.transceivers") subtitle=t("knowledge.receiver-specs-superheterodyne-architecture")>
      <TableSection title="接收机关键指标" headers=&["指标", "含义", "影响"] rows=RECEIVER_METRICS min_width=640 />
      <ConceptsSection title="核心概念" items=TRANSCEIVER_CONCEPTS />
      <BulletSection title="选购要点" items=BUYING_TIPS />
    </KnowledgePage>
  }
}
