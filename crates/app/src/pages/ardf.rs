//! 无线电测向 ARDF 速查。

use ham_web_core::ardf::{ARDF_BANDS, ARDF_CONCEPTS};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ArdfPage() -> impl IntoView {
  set_title(&t("无线电测向 ARDF"));
  view! {
    <KnowledgePage title=t("无线电测向 ARDF") subtitle=t("业余无线电测向（ARDF）· 竞赛频段")>
      <ConceptsSection title="核心概念" items=ARDF_CONCEPTS />
      <ConceptsSection title="常用频段" items=ARDF_BANDS />
    </KnowledgePage>
  }
}
