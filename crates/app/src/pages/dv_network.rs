//! 数字语音组网速查。

use ham_web_core::dv_network::{
  ANALOG_LINK_TIPS, ANALOG_LINKS, DMR_BASICS, DV_NETWORK_TIPS, DV_NETWORKS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DvNetworkPage() -> impl IntoView {
  set_title("shell.digital-voice");
  view! {
    <KnowledgePage title=move || t("shell.digital-voice") subtitle=move || t("knowledge.d-star-dmr-digital")>
      <TableSection title="主要网络" headers=&["网络", "厂商", "说明"] rows=DV_NETWORKS class="min-w-[520px]" />
      <ConceptsSection title="DMR 机理：时隙 / 色码 / Tier" items=DMR_BASICS />
      <BulletSection title="接入要点" items=DV_NETWORK_TIPS />
      <TableSection
        title="模拟中继联网（语音网关）"
        headers=&["网络", "厂商", "说明"]
        rows=ANALOG_LINKS
        class="min-w-[520px]"
      />
      <BulletSection title="联网接入要点" items=ANALOG_LINK_TIPS />
    </KnowledgePage>
  }
}
