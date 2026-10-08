//! 中继台建设与维护速查。

use ham_web_core::repeater_build::{REPEATER_BUILD, REPEATER_MAINT};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RepeaterBuildPage() -> impl IntoView {
  set_title("shell.repeater-building-2");
  view! {
    <KnowledgePage title=t("shell.repeater-building-2") subtitle=t("radio.site-selection-duplexers-coverage")>
      <ConceptsSection title="建设要素" items=REPEATER_BUILD />
      <BulletSection title="维护要点" items=REPEATER_MAINT />
    </KnowledgePage>
  }
}
