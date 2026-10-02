//! QSL 卡片设计速查。

use ham_web_core::qsl_card::{QSL_DESIGN_TIPS, QSL_REQUIRED};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn QslCardPage() -> impl IntoView {
  set_title(&t("QSL 卡片"));
  view! {
    <KnowledgePage title=t("QSL 卡片设计") subtitle=t("必备信息 · 设计建议")>
      <ConceptsSection title="必备信息" items=QSL_REQUIRED />
      <BulletSection title="设计建议" items=QSL_DESIGN_TIPS />
    </KnowledgePage>
  }
}
