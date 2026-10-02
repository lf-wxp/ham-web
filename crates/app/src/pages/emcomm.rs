//! 应急通信 EmComm 速查。

use ham_web_core::emcomm::{EMCOMM_CONCEPTS, EMCOMM_FREQS, EMCOMM_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn EmcommPage() -> impl IntoView {
  set_title(&t("应急通信 EmComm"));
  view! {
    <KnowledgePage title=t("应急通信 EmComm") subtitle=t("ARES 业余应急服务 · RACES 业余民防 · 应急频率")>
      <ConceptsSection title="组织与概念" items=EMCOMM_CONCEPTS />
      <ConceptsSection title="应急常用频率" items=EMCOMM_FREQS />
      <BulletSection title="操作要点" items=EMCOMM_TIPS />
    </KnowledgePage>
  }
}
