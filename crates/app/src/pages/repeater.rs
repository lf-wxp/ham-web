//! 中继台与数字网关速查。

use ham_web_core::repeater::{DIGITAL_GATEWAYS, INTERNET_GATEWAYS, REPEATER_CONCEPTS};
use leptos::prelude::*;

use super::repeater_lookup::RepeaterLookup;
use crate::components::common::{ConceptsSection, KnowledgePage, TableSection};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RepeaterPage() -> impl IntoView {
  set_title("shell.repeaters-gateways");
  view! {
    <KnowledgePage title=t("shell.repeaters-gateways") subtitle=t("radio.repeater-principles-digital-repeaters")>
      <ConceptsSection title="中继台概念" items=REPEATER_CONCEPTS />
      <TableSection
        title="数字中继与网关"
        headers=&["类型", "协议", "说明"]
        rows=DIGITAL_GATEWAYS
        min_width=560
      />
      // 中继台查询是交互组件，不是静态区块，必须保留在原来的位置。
      <RepeaterLookup />
      <ConceptsSection title="互联网网关" items=INTERNET_GATEWAYS inline=true />
    </KnowledgePage>
  }
}
