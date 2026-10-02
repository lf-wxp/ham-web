//! 天线极化速查。

use ham_web_core::polarization::{POLARIZATION_TIPS, POLARIZATION_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn PolarizationPage() -> impl IntoView {
  set_title(&t("天线极化"));
  view! {
    <KnowledgePage title=t("天线极化") subtitle=t("水平 · 垂直 · 圆极化")>
      <ConceptsSection title="极化类型" items=POLARIZATION_TYPES />
      <BulletSection title="选择与匹配要点" items=POLARIZATION_TIPS />
    </KnowledgePage>
  }
}
