//! 测量仪表速查。

use ham_web_core::meters::METERS;
use leptos::prelude::*;

use crate::components::common::{KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn MetersPage() -> impl IntoView {
  set_title(&t("测量仪表"));
  view! {
    <KnowledgePage title=t("测量仪表") subtitle=t("万用表 · 驻波表 · 功率计 · 天线分析仪")>
      <TableSection title="常用仪表" headers=&["仪表", "测量对象", "用途"] rows=METERS min_width=640 />
    </KnowledgePage>
  }
}
