use ham_web_core::bandplan::BAND_PLANS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, ProvenanceNote};
use crate::util::set_title;

use super::avoid_ranges_table::AvoidRangesTable;
use super::band_plan_card::BandPlanCard;
use super::mode_windows_table::ModeWindowsTable;
use crate::data;
use crate::i18n::t;

#[component]
pub fn BandPlanPage() -> impl IntoView {
  set_title("shell.band-plan");
  // 子段文案是 `crates/core` 正文（渲染时走 [`data::kt`]）：本页自带页头、没套
  // KnowledgePage 外壳，所以译文得自己确保在拉。
  data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.band-plan")
        subtitle=move || t("knowledge.iaru-region-3-band")
      />

      <PageContainer class="space-y-4">
        // 先给「实际能落哪一段」与「本地通联要避开哪一段」，再给逐波段的模式子段：
        // 题库那几组边界（29.51、431.9–432.24 这类）只有这两张表能直接查到。
        <ModeWindowsTable />
        <AvoidRangesTable />
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {BAND_PLANS.iter().map(|b| view! { <BandPlanCard plan=b /> }).collect_view()}
        </div>
        <ProvenanceNote domain="bandplan" />
      </PageContainer>
    </div>
  }
}
