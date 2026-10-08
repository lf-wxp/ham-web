//! 天线架设与支撑速查。

use ham_web_core::antenna_installation::{INSTALL_TIPS, SUPPORT_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaInstallationPage() -> impl IntoView {
  set_title("shell.antenna-installation");
  view! {
    <KnowledgePage title=t("shell.antenna-installation") subtitle=t("knowledge.height-guys-masts-towers")>
      <TableSection title="支撑方式" headers=&["方式", "规模", "用途"] rows=SUPPORT_TYPES min_width=560 />
      <BulletSection title="架设要点" items=INSTALL_TIPS />
    </KnowledgePage>
  }
}
