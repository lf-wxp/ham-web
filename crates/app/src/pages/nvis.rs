//! NVIS 近垂直入射天波速查。

use ham_web_core::nvis::{NVIS_CONCEPTS, NVIS_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NvisPage() -> impl IntoView {
  set_title(&t("NVIS 近垂直入射天波"));
  view! {
    <KnowledgePage title=t("NVIS 近垂直入射天波") subtitle=t("近距离盲区通信的天线技术")>
      <ConceptsSection title="核心概念" items=NVIS_CONCEPTS />
      <BulletSection title="应用与要点" items=NVIS_TIPS />
    </KnowledgePage>
  }
}
