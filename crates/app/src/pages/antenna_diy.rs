//! 天线 DIY 制作速查。

use ham_web_core::antenna_diy::DIY_ANTENNAS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::components::nec_template_link::NecTemplateLink;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn AntennaDiyPage() -> impl IntoView {
  set_title("shell.antenna-diy");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.antenna-diy")
        subtitle=move || t("knowledge.dipole-gp-j-pole")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.classic-home-built-antennas")}</h2>
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
                      <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{name}</TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{material}</TableCell>
                      <TableCell class=format!("{CELL} font-mono tabular-nums")>{size}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{desc}</TableCell>
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
