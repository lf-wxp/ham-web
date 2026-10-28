//! 天线 DIY 制作速查。

use ham_web_core::antenna_diy::DIY_ANTENNAS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::components::nec_template_link::NecTemplateLink;
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn AntennaDiyPage() -> impl IntoView {
  set_title("shell.antenna-diy");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.antenna-diy")
        subtitle=move || t("knowledge.dipole-gp-j-pole")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.classic-home-built-antennas")}</h2>
          <Table class="min-w-[820px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class=CELL>{move || t("log.antenna")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.materials")}</TableHeader>
                <TableHeader class=CELL>{move || t("knowledge.dimension-estimates")}</TableHeader>
                <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
                <TableHeader class=CELL>{move || t("tools.nec-model-column")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {DIY_ANTENNAS
                .iter()
                .map(|&(name, material, size, desc, template)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap")>{move || kt_tracked(name)}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(material)}</TableCell>
                      <TableCell class=format!("{CELL} font-mono tabular-nums")>{move || kt_tracked(size)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
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
      </PageContainer>
    </div>
  }
}
