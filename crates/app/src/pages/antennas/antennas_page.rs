use ham_web_core::antennas::{ANTENNA_THEORY, ANTENNAS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::antenna_card::AntennaCard;
use crate::components::common::ConceptsSection;
use crate::data;
use crate::i18n::t;

#[component]
pub fn AntennasPage() -> impl IntoView {
  set_title("shell.antenna-types");
  // 卡片与理论要点都是 `crates/core` 正文（渲染时走 [`data::kt`]）：本页自带页头、
  // 没套 KnowledgePage 外壳，所以译文得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.antenna-types")
        subtitle=move || t("knowledge.common-amateur-antennas-diagrams")
      />

      <PageContainer class="space-y-0">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {ANTENNAS.iter().map(|a| view! { <AntennaCard entry=a /> }).collect_view()}
        </div>

        <div class="mt-6">
          <ConceptsSection title="天线理论要点" items=ANTENNA_THEORY />
        </div>
      </PageContainer>
    </div>
  }
}
