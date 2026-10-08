//! 无线电测向 ARDF 速查。

use ham_web_core::ardf::{ARDF_BANDS, ARDF_CONCEPTS};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ArdfPage() -> impl IntoView {
  set_title("shell.ardf-2");
  view! {
    <KnowledgePage title=t("shell.ardf-2") subtitle=t("knowledge.amateur-radio-direction-finding")>
      <ConceptsSection title="核心概念" items=ARDF_CONCEPTS />
      <ConceptsSection title="常用频段" items=ARDF_BANDS />
    </KnowledgePage>
  }
}
