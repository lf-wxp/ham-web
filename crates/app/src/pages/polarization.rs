//! 天线极化速查。

use ham_web_core::polarization::{POLARIZATION_TIPS, POLARIZATION_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn PolarizationPage() -> impl IntoView {
  set_title("shell.polarization");
  view! {
    <KnowledgePage title=t("shell.polarization") subtitle=t("knowledge.horizontal-vertical-circular-polarisation")>
      <ConceptsSection title="极化类型" items=POLARIZATION_TYPES />
      <BulletSection title="选择与匹配要点" items=POLARIZATION_TIPS />
    </KnowledgePage>
  }
}
