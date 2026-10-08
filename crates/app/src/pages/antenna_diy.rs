//! 天线 DIY 制作速查。

use ham_web_core::antenna_diy::DIY_ANTENNAS;
use leptos::prelude::*;

use crate::components::nec_template_link::NecTemplateLink;
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaDiyPage() -> impl IntoView {
  set_title("shell.antenna-diy");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.antenna-diy")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.dipole-gp-j-pole")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.classic-home-built-antennas")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[820px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("log.antenna")}</th>
                  <th class=CELL>{move || t("knowledge.materials")}</th>
                  <th class=CELL>{move || t("knowledge.dimension-estimates")}</th>
                  <th class=CELL>{move || t("radio.notes")}</th>
                  <th class=CELL>{move || t("tools.nec-model-column")}</th>
                </tr>
              </thead>
              <tbody>
                {DIY_ANTENNAS
                  .iter()
                  .map(|&(name, material, size, desc, template)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{material}</td>
                        <td class=format!("{CELL} font-mono tabular-nums")>{size}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
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
      </div>
    </div>
  }
}
