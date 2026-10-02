//! IOTA 海岛通联速查。

use ham_web_core::iota::{IOTA_CONCEPTS, IOTA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn IotaPage() -> impl IntoView {
  set_title(&t("IOTA 海岛通联"));
  view! {
    <KnowledgePage title=t("IOTA 海岛通联") subtitle=t("Islands On The Air · 岛组奖状")>
      <ConceptsSection title="核心概念" items=IOTA_CONCEPTS />
      <BulletSection title="参与要点" items=IOTA_TIPS />
    </KnowledgePage>
  }
}
