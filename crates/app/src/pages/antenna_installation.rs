//! 天线架设与支撑速查。

use ham_web_core::antenna_installation::{INSTALL_TIPS, SUPPORT_TYPES};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaInstallationPage() -> impl IntoView {
  set_title(&t("天线架设"));
  view! {
    <KnowledgePage title=t("天线架设") subtitle=t("高度 · 拉线 · 桅杆/塔 · 安全")>
      <TableSection title="支撑方式" headers=&["方式", "规模", "用途"] rows=SUPPORT_TYPES min_width=560 />
      <BulletSection title="架设要点" items=INSTALL_TIPS />
    </KnowledgePage>
  }
}
