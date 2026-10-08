//! NVIS 近垂直入射天波速查。

use ham_web_core::nvis::{NVIS_CONCEPTS, NVIS_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn NvisPage() -> impl IntoView {
  set_title("shell.nvis");
  view! {
    <KnowledgePage title=t("shell.nvis") subtitle=t("radio.antenna-techniques-for-close")>
      <ConceptsSection title="核心概念" items=NVIS_CONCEPTS />
      <BulletSection title="应用与要点" items=NVIS_TIPS />
    </KnowledgePage>
  }
}
