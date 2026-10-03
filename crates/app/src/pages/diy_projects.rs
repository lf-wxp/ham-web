//! DIY 实战项目教程索引。

use ham_web_core::diy_projects::{DIY_PROJECTS, DIY_SAFETY, DIY_STEPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DiyProjectsPage() -> impl IntoView {
  set_title(&t("DIY 实战项目"));
  view! {
    <KnowledgePage title=t("DIY 实战项目教程") subtitle=t("从入门到进阶的动手项目与通用流程")>
      <TableSection title="项目列表" headers=&["项目", "难度", "要点"] rows=DIY_PROJECTS />
      <StepsSection title="通用制作流程" items=DIY_STEPS />
      <BulletSection title="制作安全提醒" items=DIY_SAFETY />
    </KnowledgePage>
  }
}
