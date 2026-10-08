//! DX 通联技巧速查。

use ham_web_core::dx::{DX_CONCEPTS, DX_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn DxPage() -> impl IntoView {
  set_title("shell.dx-techniques");
  view! {
    <KnowledgePage title=t("shell.dx-techniques") subtitle=t("radio.split-operation-dx-window")>
      <ConceptsSection title="核心概念" items=DX_CONCEPTS />
      <BulletSection title="追台要点" items=DX_TIPS />
    </KnowledgePage>
  }
}
