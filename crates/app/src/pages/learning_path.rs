//! 学习路径与认证路线图速查。

use ham_web_core::learning_path::{LEARNING_LEVELS, LEARNING_PATH, LEARNING_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn LearningPathPage() -> impl IntoView {
  set_title(&t("学习路径"));
  view! {
    <KnowledgePage title=t("学习路径与认证路线图") subtitle=t("从新手到 A/B/C 操作证的循序渐进指引")>
      <StepsSection title="学习路径" items=LEARNING_PATH />
      <TableSection title="各级别重点" headers=&["级别", "重点", "对应页面"] rows=LEARNING_LEVELS />
      <BulletSection title="备考建议" items=LEARNING_TIPS />
    </KnowledgePage>
  }
}
