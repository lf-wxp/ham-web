//! 车载 / 移动电台：安装、电源与噪声抑制速查。

use ham_web_core::mobile::{MOBILE_CONCEPTS, MOBILE_INSTALL, MOBILE_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn MobilePage() -> impl IntoView {
  set_title("shell.mobile-stations");
  view! {
    <KnowledgePage title=t("shell.mobile-stations") subtitle=t("radio.mobile-installation-power-wiring")>
      <ConceptsSection title="核心概念" items=MOBILE_CONCEPTS />
      <ConceptsSection title="安装要点" items=MOBILE_INSTALL />
      <BulletSection title="操作要点" items=MOBILE_TIPS />
    </KnowledgePage>
  }
}
