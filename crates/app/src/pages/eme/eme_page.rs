//! EME 月面反射：月球位置 / 窗口计划器 + 知识速查。

use ham_web_core::eme::{EME_CONCEPTS, EME_REQUIREMENTS, EME_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

use super::eme_planner::EmePlanner;

#[component]
pub fn EmePage() -> impl IntoView {
  set_title("shell.eme-moonbounce");
  view! {
    <KnowledgePage title=move || t("shell.eme-moonbounce") subtitle=move || t("radio.antenna-arrays-power-weak")>
      <EmePlanner />
      <ConceptsSection title="核心概念" items=EME_CONCEPTS />
      <TableSection title="设备要求" headers=&["项目", "要求", "说明"] rows=EME_REQUIREMENTS class="min-w-[560px]" />
      <BulletSection title="操作要点" items=EME_TIPS />
    </KnowledgePage>
  }
}
