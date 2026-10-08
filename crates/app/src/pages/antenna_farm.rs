//! 天线农场规划速查。

use ham_web_core::antenna_farm::{FARM_FACTORS, FARM_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaFarmPage() -> impl IntoView {
  set_title("shell.antenna-farms-2");
  view! {
    <KnowledgePage title=t("shell.antenna-farms-2") subtitle=t("knowledge.multi-antenna-layouts-isolation")>
      <ConceptsSection title="规划要素" items=FARM_FACTORS />
      <BulletSection title="实践要点" items=FARM_TIPS />
    </KnowledgePage>
  }
}
