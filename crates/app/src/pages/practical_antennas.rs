//! 实用天线专题速查：EFHW、磁环小环、接收天线与倒 V。

use ham_web_core::practical_antennas::{
  PRACTICAL_ANTENNAS_CONCEPTS, PRACTICAL_ANTENNAS_TABLE, PRACTICAL_ANTENNAS_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::components::nec_template_link::NecTemplateLink;
use crate::i18n::t;

const CELL: &str = "border px-3 py-2 text-left align-top";
use crate::util::set_title;

#[component]
pub fn PracticalAntennasPage() -> impl IntoView {
  set_title("knowledge.practical-antenna-topics");
  view! {
    <KnowledgePage title=t("knowledge.practical-antenna-topics") subtitle=t("knowledge.how-to-choose-between")>
      <ConceptsSection title="核心概念" items=PRACTICAL_ANTENNAS_CONCEPTS />
      // 对比表比通用三列表格多一列「求解器模型」：能建模的方案直接跳到 `/nec`，
      // 不能的（小环、Beverage、K9AY）如实写「暂无可用模型」而不是硬凑一个。
      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">
          {move || t("knowledge.antenna-comparison")}
        </h2>
        <div class="overflow-x-auto">
          <table class="w-full min-w-[820px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("log.antenna")}</th>
                <th class=CELL>{move || t("knowledge.features")}</th>
                <th class=CELL>{move || t("knowledge.best-for")}</th>
                <th class=CELL>{move || t("tools.nec-model-column")}</th>
              </tr>
            </thead>
            <tbody>
              {PRACTICAL_ANTENNAS_TABLE
                .iter()
                .map(|&(name, feature, scene, template)| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>
                        {crate::data::kt(name)}
                      </td>
                      <td class=format!("{CELL} text-muted-foreground")>{feature}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{scene}</td>
                      <td class=CELL>
                        <NecTemplateLink template=template />
                      </td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>
      </section>
      <BulletSection title="制作与使用要点" items=PRACTICAL_ANTENNAS_TIPS />
    </KnowledgePage>
  }
}
