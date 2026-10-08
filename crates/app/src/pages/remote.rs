//! 远程电台 Remote Station 速查。

use ham_web_core::remote::{REMOTE_CONCEPTS, REMOTE_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RemotePage() -> impl IntoView {
  set_title("shell.remote-stations");
  view! {
    <KnowledgePage title=t("shell.remote-stations") subtitle=t("radio.remote-operation-control-protocols")>
      <ConceptsSection title="核心概念" items=REMOTE_CONCEPTS />
      <BulletSection title="搭建要点" items=REMOTE_TIPS />
    </KnowledgePage>
  }
}
