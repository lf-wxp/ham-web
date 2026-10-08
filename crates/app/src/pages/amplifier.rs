//! 功率放大器（PA）速查。

use ham_web_core::amplifier::{PA_CONCEPTS, PA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AmplifierPage() -> impl IntoView {
  set_title("shell.amplifiers");
  view! {
    <KnowledgePage title=t("shell.amplifiers") subtitle=t("knowledge.power-amplification-linearity-imd")>
      <ConceptsSection title="核心概念" items=PA_CONCEPTS />
      <BulletSection title="使用要点" items=PA_TIPS />
    </KnowledgePage>
  }
}
