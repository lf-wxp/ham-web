//! WSPR 弱信号传播报告速查。

use ham_web_core::wspr::{WSPR_CONCEPTS, WSPR_NOTES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn WsprPage() -> impl IntoView {
  set_title(&t("WSPR 弱信号传播"));
  view! {
    <KnowledgePage title=t("WSPR 弱信号传播报告") subtitle=t("弱信号传播报告 · 传播研究")>
      <ConceptsSection title="核心概念" items=WSPR_CONCEPTS />
      <BulletSection title="使用要点" items=WSPR_NOTES />
    </KnowledgePage>
  }
}
