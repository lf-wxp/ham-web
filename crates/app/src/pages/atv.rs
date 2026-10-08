//! 业余电视 ATV / DATV 速查。

use ham_web_core::atv::{ATV_CONCEPTS, ATV_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AtvPage() -> impl IntoView {
  set_title("shell.amateur-tv-2");
  view! {
    <KnowledgePage title=t("shell.amateur-tv-2") subtitle=t("knowledge.atv-amateur-television-datv")>
      <ConceptsSection title="概念与类型" items=ATV_CONCEPTS />
      <BulletSection title="操作要点" items=ATV_TIPS />
    </KnowledgePage>
  }
}
