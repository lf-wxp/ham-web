//! 数字语音组网速查。

use ham_web_core::dv_network::{DV_NETWORK_TIPS, DV_NETWORKS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DvNetworkPage() -> impl IntoView {
  set_title("shell.digital-voice");
  view! {
    <KnowledgePage title=t("shell.digital-voice") subtitle=t("knowledge.d-star-dmr-digital")>
      <TableSection title="主要网络" headers=&["网络", "厂商", "说明"] rows=DV_NETWORKS min_width=520 />
      <BulletSection title="接入要点" items=DV_NETWORK_TIPS />
    </KnowledgePage>
  }
}
