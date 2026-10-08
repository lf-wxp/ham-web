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
      title=t("knowledge.cables-and-connectors")
      subtitle=t("knowledge.cable-selection-rf-connectors")
    >
      <TableSection
        title=t("knowledge.rf-connector-comparison")
        headers=&[
          "knowledge.model",
          "knowledge.impedance-and-frequency-limit",
          "knowledge.features-and-typical-use",
        ]
        rows=CONNECTORS
        min_width=720
      />
      <BulletSection title=t("knowledge.connector-selection-tips") items=SELECTION_TIPS />
      <TableSection
        title=t("knowledge.common-cable-selection")
        headers=&[
          "knowledge.model",
          "knowledge.structure-and-shielding",
          "knowledge.selection-advice",
        ]
        rows=CABLES
        min_width=640
      />
      <BulletSection
        title=t("knowledge.connector-installation-and-waterproofing")
        items=INSTALL_TIPS
      />
      <BulletSection title=t("knowledge.common-faults-and-troubleshooting") items=FAULTS />
    </KnowledgePage>
  }
}
