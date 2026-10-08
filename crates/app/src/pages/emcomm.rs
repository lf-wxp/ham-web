//! 应急通信 EmComm 速查。

use ham_web_core::emcomm::{EMCOMM_CONCEPTS, EMCOMM_FREQS, EMCOMM_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn EmcommPage() -> impl IntoView {
  set_title("shell.emergency-comms-2");
  view! {
    <KnowledgePage title=t("shell.emergency-comms-2") subtitle=t("knowledge.ares-amateur-emergency-service")>
      <ConceptsSection title="组织与概念" items=EMCOMM_CONCEPTS />
      <ConceptsSection title="应急常用频率" items=EMCOMM_FREQS />
      <BulletSection title="操作要点" items=EMCOMM_TIPS />
    </KnowledgePage>
  }
}
