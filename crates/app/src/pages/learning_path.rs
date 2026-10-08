//! 学习路径与认证路线图速查。

use ham_web_core::learning_path::{LEARNING_LEVELS, LEARNING_PATH, LEARNING_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn LearningPathPage() -> impl IntoView {
  set_title("knowledge.learning-path");
  view! {
    <KnowledgePage title=t("knowledge.learning-path-and-certification") subtitle=t("knowledge.a-step-by-step")>
      <StepsSection title="学习路径" items=LEARNING_PATH />
      <TableSection title="各级别重点" headers=&["级别", "重点", "对应页面"] rows=LEARNING_LEVELS />
      <BulletSection title="备考建议" items=LEARNING_TIPS />
    </KnowledgePage>
  }
}
