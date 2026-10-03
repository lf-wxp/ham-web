//! 流星散射速查。

use ham_web_core::meteor_scatter::{METEOR_SHOWERS, MS_CONCEPTS, MS_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn MeteorScatterPage() -> impl IntoView {
  set_title(&t("流星散射"));
  view! {
    <KnowledgePage title=t("流星散射") subtitle=t("6m / 2m 弱信号 DX 的主力手段")>
      <ConceptsSection title="核心概念" items=MS_CONCEPTS />
      <TableSection title="主要流星雨" headers=&["流星雨", "峰值时段", "说明"] rows=METEOR_SHOWERS min_width=640 />
      <BulletSection title="操作要点" items=MS_TIPS />
    </KnowledgePage>
  }
}
