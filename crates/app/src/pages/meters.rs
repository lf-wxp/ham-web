//! 测量仪表速查。

use ham_web_core::meters::METERS;
use leptos::prelude::*;

use crate::components::common::{KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn MetersPage() -> impl IntoView {
  set_title("shell.test-instruments");
  view! {
    <KnowledgePage title=t("shell.test-instruments") subtitle=t("tools.multimeters-swr-meters-power")>
      <TableSection title="常用仪表" headers=&["仪表", "测量对象", "用途"] rows=METERS min_width=640 />
    </KnowledgePage>
  }
}
