use ham_web_core::antenna_modeling::{MODELING_SOFTWARE, MODELING_STEPS, MODELING_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

use super::dipole_calculator::DipoleCalculator;
use super::pattern_plot::PatternPlot;
use super::vertical_calculator::VerticalCalculator;
use super::yagi_calculator::YagiCalculator;
use crate::data;
use crate::i18n::t;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaModelingPage() -> impl IntoView {
  set_title("shell.antenna-modeling-title");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.antenna-modeling-title")
        subtitle="EZNEC · MMANA-GAL · 4NEC2".to_string()
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.radiation-pattern-visualisation")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.polar-plots-show-the")}
          </p>
          <div class="p-4"><PatternPlot /></div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.dipole-dimension-estimate")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.estimate-the-size-before")}
          </p>
          <div class="p-4"><DipoleCalculator /></div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.yagi-antenna-dimensions")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("knowledge.the-reflector-is-slightly")}
          </p>
          <div class="p-4"><YagiCalculator /></div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.vertical-antenna-dimensions")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("knowledge.quarter-wave-vertical-radials")}
          </p>
          <div class="p-4"><VerticalCalculator /></div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.related-calculators")}</h2>
          <div class="flex flex-wrap gap-2 p-4">
            {[
              ("/tools#antenna-length", "天线长度"),
              ("/tools#coil-yagi", "线圈 / Yagi 振子"),
              ("/tools#swr", "驻波比换算"),
              ("/tools#feedline-loss", "馈线损耗"),
              ("/tools#gain-conversion", "增益换算"),
            ]
              .iter()
              .map(|(href, label)| {
                view! {
                  <a
                    href=*href
                    class="border bg-inset px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                  >
                    {move || crate::data::core_text_tracked(label)}
                  </a>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.common-software")}</h2>
          <Table class="min-w-[520px] text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class="border px-3 py-2 align-top">{move || t("radio.software")}</TableHeader>
                <TableHeader class="border px-3 py-2 align-top">{move || t("radio.type")}</TableHeader>
                <TableHeader class="border px-3 py-2 align-top">{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {MODELING_SOFTWARE
                .iter()
                .map(|&(name, kind, desc)| {
                  view! {
                    <TableRow>
                      <TableCell class=format!("{CELL} whitespace-nowrap font-mono text-primary")>
                        {move || kt_tracked(name)}
                      </TableCell>
                      <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(kind)}</TableCell>
                      <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.modelling-workflow")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MODELING_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.usage-notes")}</h2>
          <ul class="space-y-2 p-4">
            {MODELING_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </PageContainer>
    </div>
  }
}
