//! 无线电测向 ARDF 速查，附「几条方位线 → 一个位置」的交汇定位面板。

mod df_panel;
mod df_row;

use ham_web_core::ardf::{ARDF_BANDS, ARDF_CONCEPTS};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;
use df_panel::DfPanel;

#[component]
pub fn ArdfPage() -> impl IntoView {
  set_title("shell.ardf-title");
  view! {
    <KnowledgePage title=move || t("shell.ardf-title") subtitle=move || t("knowledge.amateur-radio-direction-finding")>
      <ConceptsSection title="核心概念" items=ARDF_CONCEPTS />
      <ConceptsSection title="常用频段" items=ARDF_BANDS />
      <DfPanel />
    </KnowledgePage>
  }
}
