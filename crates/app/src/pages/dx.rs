//! DX 通联技巧速查。

use ham_web_core::dx::{DX_CONCEPTS, DX_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DxPage() -> impl IntoView {
  set_title(&t("DX 远距离通信技巧"));
  view! {
    <KnowledgePage title=t("DX 远距离通信技巧") subtitle=t("分裂操作 · DX 窗口 · Pileup 礼仪")>
      <ConceptsSection title="核心概念" items=DX_CONCEPTS />
      <BulletSection title="追台要点" items=DX_TIPS />
    </KnowledgePage>
  }
}
