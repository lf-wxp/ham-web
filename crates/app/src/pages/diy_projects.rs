//! DIY 实战项目教程索引。

use ham_web_core::diy_projects::{DIY_PROJECTS, DIY_SAFETY, DIY_STEPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DiyProjectsPage() -> impl IntoView {
  set_title("knowledge.hands-on-diy-projects");
  view! {
    <KnowledgePage title=t("knowledge.hands-on-diy-project") subtitle=t("knowledge.hands-on-projects-from")>
      <TableSection title="项目列表" headers=&["项目", "难度", "要点"] rows=DIY_PROJECTS />
      <StepsSection title="通用制作流程" items=DIY_STEPS />
      <BulletSection title="制作安全提醒" items=DIY_SAFETY />
    </KnowledgePage>
  }
}
