//! 软件定义无线电 SDR 速查。

use ham_web_core::sdr::{SDR_CONCEPTS, SDR_SOFTWARE, WEB_SDR};
use leptos::prelude::*;

use crate::components::common::{ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SdrPage() -> impl IntoView {
  set_title(&t("软件定义无线电 SDR"));
  view! {
    <KnowledgePage title=t("软件定义无线电 SDR") subtitle=t("概念 · 架构 · 常用软件")>
      <ConceptsSection title="核心概念" items=SDR_CONCEPTS />
      <ConceptsSection title="常用软件" items=SDR_SOFTWARE />
      <ConceptsSection title="在线收听（无需本地硬件）" items=WEB_SDR />
    </KnowledgePage>
  }
}
