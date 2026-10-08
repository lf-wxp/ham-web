//! EME 月面反射速查。

use ham_web_core::eme::{EME_CONCEPTS, EME_REQUIREMENTS, EME_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn EmePage() -> impl IntoView {
  set_title("shell.eme-moonbounce");
  view! {
    <KnowledgePage title=t("shell.eme-moonbounce") subtitle=t("radio.antenna-arrays-power-weak")>
      <ConceptsSection title="核心概念" items=EME_CONCEPTS />
      <TableSection title="设备要求" headers=&["项目", "要求", "说明"] rows=EME_REQUIREMENTS min_width=560 />
      <BulletSection title="操作要点" items=EME_TIPS />
    </KnowledgePage>
  }
}
