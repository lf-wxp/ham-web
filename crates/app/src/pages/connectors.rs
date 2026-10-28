//! 线材与连接器速查。

use ham_web_core::connectors::{CABLES, CONNECTORS, FAULTS, INSTALL_TIPS, SELECTION_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ConnectorsPage() -> impl IntoView {
  set_title("knowledge.cables-and-connectors");
  view! {
    <KnowledgePage
      title=move || t("knowledge.cables-and-connectors")
      subtitle=move || t("knowledge.cable-selection-rf-connectors")
    >
      <TableSection
        title="knowledge.rf-connector-comparison"
        headers=&[
          "knowledge.model",
          "knowledge.impedance-and-frequency-limit",
          "knowledge.features-and-typical-use",
        ]
        rows=CONNECTORS
        class="min-w-[720px]"
      />
      <BulletSection title="knowledge.connector-selection-tips" items=SELECTION_TIPS />
      <TableSection
        title="knowledge.common-cable-selection"
        headers=&[
          "knowledge.model",
          "knowledge.structure-and-shielding",
          "knowledge.selection-advice",
        ]
        rows=CABLES
        class="min-w-[640px]"
      />
      <BulletSection
        title="knowledge.connector-installation-and-waterproofing"
        items=INSTALL_TIPS
      />
      <BulletSection title="knowledge.common-faults-and-troubleshooting" items=FAULTS />
    </KnowledgePage>
  }
}
