//! Winlink 无线邮件系统速查。

use ham_web_core::winlink::{WINLINK_CONCEPTS, WINLINK_MODES, WINLINK_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn WinlinkPage() -> impl IntoView {
  set_title("knowledge.winlink-radio-email");
  view! {
    <KnowledgePage title=t("knowledge.winlink-radio-email") subtitle=t("knowledge.send-and-receive-email")>
      <ConceptsSection title="核心概念" items=WINLINK_CONCEPTS />
      <TableSection title="常用调制方式" headers=&["方式", "类型", "说明"] rows=WINLINK_MODES min_width=640 />
      <BulletSection title="应急用法与要点" items=WINLINK_TIPS />
    </KnowledgePage>
  }
}
