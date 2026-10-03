//! 学习资源索引：视频课程、技术文档与自学建议。

use ham_web_core::learning_resources::{DOC_RESOURCES, LEARNING_RESOURCES_TIPS, VIDEO_RESOURCES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn LearningResourcesPage() -> impl IntoView {
  set_title(&t("学习资源"));
  view! {
    <KnowledgePage title=t("学习资源") subtitle=t("视频课程、技术文档与自学建议")>
      <TableSection title="视频资源" headers=&["平台", "内容", "建议"] rows=VIDEO_RESOURCES />
      <TableSection title="技术文档" headers=&["文档", "内容", "用途"] rows=DOC_RESOURCES />
      <BulletSection title="自学建议" items=LEARNING_RESOURCES_TIPS />
    </KnowledgePage>
  }
}
