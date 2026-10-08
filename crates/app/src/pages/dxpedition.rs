//! DX 远征（DXpedition）速查。

use ham_web_core::dxpedition::{DXPED_CONCEPTS, DXPED_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DxpeditionPage() -> impl IntoView {
  set_title("radio.dxpeditions");
  view! {
    <KnowledgePage title=t("radio.dxpeditions") subtitle=t("radio.dxpeditions-and-chasing-rare")>
      <ConceptsSection title="核心概念" items=DXPED_CONCEPTS />
      <BulletSection title="追远征台要点" items=DXPED_TIPS />
    </KnowledgePage>
  }
}
