use ham_web_core::antenna_modeling::{MODELING_SOFTWARE, MODELING_STEPS, MODELING_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

use super::dipole_calculator::DipoleCalculator;
use super::pattern_plot::PatternPlot;
use super::vertical_calculator::VerticalCalculator;
use super::yagi_calculator::YagiCalculator;
use crate::i18n::t;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaModelingPage() -> impl IntoView {
  set_title("shell.antenna-modeling-2");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.antenna-modeling-2")
        subtitle="EZNEC · MMANA-GAL · 4NEC2".to_string()
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.radiation-pattern-visualisation")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.polar-plots-show-the")}
          </p>
          <div class="p-4"><PatternPlot /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.dipole-dimension-estimate")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.estimate-the-size-before")}
          </p>
          <div class="p-4"><DipoleCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.yagi-antenna-dimensions")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("knowledge.the-reflector-is-slightly")}
          </p>
          <div class="p-4"><YagiCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.vertical-antenna-dimensions")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("knowledge.quarter-wave-vertical-radials")}
          </p>
          <div class="p-4"><VerticalCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.related-calculators")}</h2>
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
                    class="rounded-full border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                  >
                    {move || t(label)}
                  </a>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.common-software")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[520px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("radio.software")}</th>
                  <th class=CELL>{move || t("radio.type")}</th>
                  <th class=CELL>{move || t("radio.notes")}</th>
                </tr>
              </thead>
              <tbody>
                {MODELING_SOFTWARE
                  .iter()
                  .map(|&(name, kind, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {name}
                        </td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{kind}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.modelling-workflow")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MODELING_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.usage-notes")}</h2>
          <ul class="space-y-2 p-4">
            {MODELING_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
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
