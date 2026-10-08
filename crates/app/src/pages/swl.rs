//! SWL 短波监听速查。

use ham_web_core::swl::{SWL_BANDS, SWL_TARGETS, SWL_TIPS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SwlPage() -> impl IntoView {
  set_title("shell.swl");
  view! {
    <KnowledgePage title=t("shell.swl") subtitle=t("knowledge.shortwave-broadcast-data-decoding")>
      <TableSection title="可监听内容" headers=&["名称", "类别", "说明"] rows=SWL_TARGETS min_width=520 />
      <ConceptsSection title="常用短波广播频段" items=SWL_BANDS inline=true />
      <BulletSection title="入门要点" items=SWL_TIPS />
    </KnowledgePage>
  }
}
