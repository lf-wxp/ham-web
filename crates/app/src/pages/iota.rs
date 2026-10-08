//! IOTA 海岛通联速查。

use ham_web_core::iota::{IOTA_CONCEPTS, IOTA_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn IotaPage() -> impl IntoView {
  set_title("shell.iota");
  view! {
    <KnowledgePage title=t("shell.iota") subtitle=t("radio.islands-on-the-air")>
      <ConceptsSection title="核心概念" items=IOTA_CONCEPTS />
      <BulletSection title="参与要点" items=IOTA_TIPS />
    </KnowledgePage>
  }
}
