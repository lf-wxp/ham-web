//! 功率放大器（PA）速查。

use ham_web_core::amplifier::{PA_CONCEPTS, PA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AmplifierPage() -> impl IntoView {
  set_title(&t("功率放大器"));
  view! {
    <KnowledgePage title=t("功率放大器") subtitle=t("功率放大 · 线性度 · IMD")>
      <ConceptsSection title="核心概念" items=PA_CONCEPTS />
      <BulletSection title="使用要点" items=PA_TIPS />
    </KnowledgePage>
  }
}
