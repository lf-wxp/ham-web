//! QSL 卡片设计速查。

use ham_web_core::qsl_card::{QSL_DESIGN_TIPS, QSL_REQUIRED};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn QslCardPage() -> impl IntoView {
  set_title("shell.qsl-card");
  view! {
    <KnowledgePage title=t("shell.qsl-card-design") subtitle=t("log.essential-information-design-advice")>
      <ConceptsSection title="必备信息" items=QSL_REQUIRED />
      <BulletSection title="设计建议" items=QSL_DESIGN_TIPS />
    </KnowledgePage>
  }
}
