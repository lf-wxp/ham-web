//! 实用天线专题速查：EFHW、磁环小环、接收天线与倒 V。

use ham_web_core::practical_antennas::{
  PRACTICAL_ANTENNAS_CONCEPTS, PRACTICAL_ANTENNAS_TABLE, PRACTICAL_ANTENNAS_TIPS,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::components::nec_template_link::NecTemplateLink;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

const CELL: &str = "border px-3 py-2 align-top";
use crate::util::set_title;

#[component]
pub fn PracticalAntennasPage() -> impl IntoView {
  set_title("knowledge.practical-antenna-topics");
  view! {
    <KnowledgePage title=t("knowledge.practical-antenna-topics") subtitle=t("knowledge.how-to-choose-between")>
      <ConceptsSection title="核心概念" items=PRACTICAL_ANTENNAS_CONCEPTS />
      // 对比表比通用三列表格多一列「求解器模型」：能建模的方案直接跳到 `/nec`，
      // 不能的（小环、Beverage、K9AY）如实写「暂无可用模型」而不是硬凑一个。
      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">
          {move || t("knowledge.antenna-comparison")}
        </h2>
        <Table class="min-w-[820px] text-sm">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("log.antenna")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.features")}</TableHeader>
              <TableHeader class=CELL>{move || t("knowledge.best-for")}</TableHeader>
              <TableHeader class=CELL>{move || t("tools.nec-model-column")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {PRACTICAL_ANTENNAS_TABLE
              .iter()
              .map(|&(name, feature, scene, template)| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>
                      {move || text(name)}
                    </TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{move || text(feature)}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{move || text(scene)}</TableCell>
                    <TableCell class=CELL>
                      <NecTemplateLink template=template />
                    </TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>
      </section>
      <BulletSection title="制作与使用要点" items=PRACTICAL_ANTENNAS_TIPS />
    </KnowledgePage>
  }
}

/// 知识库正文译文：先订阅加载状态（译文异步到达后这一格要重算），再查词典。
///
/// 三列（名称 / 特点 / 适用场景）都来自 `crates/core` 的中文正文，直接渲染的话
/// 英文 / 西语界面整张表都是中文 —— 逐格换成这个入口，译文一旦补上就即时生效。
fn text(zh: &'static str) -> String {
  crate::data::track_knowledge();
  crate::data::kt(zh)
}
