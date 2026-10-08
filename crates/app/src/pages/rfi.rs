//! 射频干扰与电磁兼容（RFI / EMC）速查。

use ham_web_core::rfi::{RFI_SOLUTIONS, RFI_SOURCES, RFI_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RfiPage() -> impl IntoView {
  set_title("shell.rfi-troubleshooting");
  view! {
    <KnowledgePage title=t("shell.rfi-troubleshooting") subtitle=t("tools.rfi-emc-tracing-and")>
      <ConceptsSection title="常见干扰源" items=RFI_SOURCES />
      <ConceptsSection title="排查与抑制手段" items=RFI_SOLUTIONS />
      <BulletSection title="处理要点" items=RFI_TIPS />
    </KnowledgePage>
  }
}
