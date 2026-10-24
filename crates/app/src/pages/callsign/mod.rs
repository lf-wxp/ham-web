//! 呼号知识总页：结构、查询、台站类别、分区、斜杠后缀与分区地图。
//!
//! 呼号相关的知识原本分散在 `/prefixes`（结构图解与两张说明表）与 `/callsign`
//! （查询、分区地图），两边的台站类别文案还是各写一份。现在统一收在这里：
//! 数据留在 `ham_web_core::callsign`，页面只负责排版。

mod callsign_lookup;
mod callsign_reference;
mod callsign_structure;

use ham_web_core::callsign::CALLSIGN_TIPS;
use ham_web_core::dxcc::lookup;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::common::{BulletSection, KnowledgePage};
use crate::i18n::t;
use crate::pages::map::ZoneMap;
use crate::ui::{Chip, ChipGroup, card_title_class};
use crate::util::set_title;

use callsign_lookup::CallsignLookup;
use callsign_reference::CallsignReference;
use callsign_structure::CallsignStructure;

#[component]
pub fn CallsignPage() -> impl IntoView {
  // 页面叫「呼号知识」：查询只是其中一块，页头不宜再只提查询。
  // 工具页里的「呼号查询」计算器仍用 `tools.callsign-lookup`。
  set_title("tools.callsign-knowledge");

  let query = use_query_map();
  let input = RwSignal::new(query.with_untracked(|q| q.get("call")).unwrap_or_default());
  // 分区地图：默认 CQ，高亮当前呼号所属实体。
  let is_cq = RwSignal::new(true);
  let zone_dxcc = Signal::derive(move || {
    let call = input.get().trim().to_ascii_uppercase();
    if call.is_empty() {
      return None;
    }
    lookup(&call).map(|e| e.dxcc)
  });

  view! {
    <KnowledgePage
      title=t("tools.callsign-knowledge")
      subtitle=t("radio.callsign-knowledge-hub")
    >
      <CallsignStructure />
      <CallsignLookup input=input />
      <CallsignReference />

      <section class="pxl-card border bg-card">
        <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
          <h2 class=card_title_class("mr-auto")>{move || t("radio.zone-map-cq-itu")}</h2>
          <ChipGroup
            value=Signal::derive(move || (if is_cq.get() { "cq" } else { "itu" }).to_string())
            on_change=Callback::new(move |v: String| is_cq.set(v == "cq"))
            aria_label=Signal::derive(move || t("radio.zone-map-cq-itu"))
          >
            {
              let cq = view! { {move || t("common.cq-zone")} }.into_any();
              let itu = view! { {move || t("common.itu-zone")} }.into_any();
              view! {
                <Chip value="cq".to_string()>{cq}</Chip>
                <Chip value="itu".to_string()>{itu}</Chip>
              }
            }
          </ChipGroup>
        </div>
        <div class="p-4">
          <ZoneMap is_cq=is_cq highlight=zone_dxcc />
        </div>
        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("radio.coloured-by-each-dxcc")}
          <a
            href=move || {
              let call = input.get().trim().to_ascii_uppercase();
              if call.is_empty() {
                "/zone-map".to_owned()
              } else {
                format!("/zone-map?q={}", js_sys::encode_uri_component(&call))
              }
            }
            class="ml-1 text-primary underline underline-offset-2"
          >
            {move || t("radio.open-the-full-zone")}
          </a>
        </p>
      </section>

      <BulletSection title="radio.how-to-use" items=CALLSIGN_TIPS />
    </KnowledgePage>
  }
}
