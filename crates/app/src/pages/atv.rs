//! 业余电视 ATV / DATV 速查。

use ham_web_core::atv::{ATV_CONCEPTS, ATV_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AtvPage() -> impl IntoView {
  set_title(&t("业余电视 ATV / DATV"));
  view! {
    <KnowledgePage title=t("业余电视 ATV / DATV") subtitle=t("ATV 业余电视 · DATV 数字业余电视")>
      <ConceptsSection title="概念与类型" items=ATV_CONCEPTS />
      <BulletSection title="操作要点" items=ATV_TIPS />
    </KnowledgePage>
  }
}
