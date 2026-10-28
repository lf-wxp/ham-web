//! 国内特色专题：国内业余卫星转发器频率表 + 公众对讲机（409 MHz）与业余手台的合规区别。

use ham_web_core::china_ham::{CHINA_HAM_CONCEPTS, CHINA_HAM_TIPS, PMR_VS_HAM, XW2_TRANSPONDERS};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage, TableSection};
use crate::data;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn ChinaHamPage() -> impl IntoView {
  set_title("knowledge.china-ham");
  view! {
    <KnowledgePage
      title=move || t("knowledge.china-ham")
      subtitle=move || t("knowledge.china-ham-subtitle")
    >
      <ConceptsSection title="核心概念" items=CHINA_HAM_CONCEPTS />

      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">{move || t("knowledge.china-ham-xw2")}</h2>
        <Table aria_label=Signal::derive(move || t("knowledge.china-ham-xw2")) class="min-w-[640px]">
          <TableHead>
            <TableRow>
              <TableHeader>{move || t("radio.satellite")}</TableHeader>
              <TableHeader>{move || t("knowledge.china-ham-downlink")}</TableHeader>
              <TableHeader>{move || t("knowledge.china-ham-uplink")}</TableHeader>
              <TableHeader>{move || t("knowledge.china-ham-beacon")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {XW2_TRANSPONDERS
              .iter()
              .map(|(name, dl, ul, bcn)| {
                view! {
                  <TableRow>
                    <TableCell class="align-top whitespace-nowrap">
                      {move || {
                        data::track_knowledge();
                        data::kt(name)
                      }}
                    </TableCell>
                    <TableCell class="align-top font-mono text-muted-foreground">
                      {move || {
                        data::track_knowledge();
                        data::kt(dl)
                      }}
                    </TableCell>
                    <TableCell class="align-top font-mono text-muted-foreground">
                      {move || {
                        data::track_knowledge();
                        data::kt(ul)
                      }}
                    </TableCell>
                    <TableCell class="align-top font-mono text-muted-foreground">
                      {move || {
                        data::track_knowledge();
                        data::kt(bcn)
                      }}
                    </TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>
      </section>

      <TableSection
        title="knowledge.china-ham-pmr-vs-ham"
        headers=&["knowledge.china-ham-dimension", "knowledge.china-ham-pmr", "knowledge.china-ham-ham"]
        rows=PMR_VS_HAM
        class="min-w-[760px]"
      />

      <BulletSection title="要点" items=CHINA_HAM_TIPS />
    </KnowledgePage>
  }
}
