//! 天线 DIY 制作速查。

use ham_web_core::antenna_diy::DIY_ANTENNAS;
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaDiyPage() -> impl IntoView {
  set_title(&t("天线 DIY"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("天线 DIY")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("偶极 · GP · J 型 · 八木 尺寸估算")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("经典自制天线")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[720px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("天线")}</th>
                  <th class=CELL>{move || t("材料")}</th>
                  <th class=CELL>{move || t("尺寸估算")}</th>
                  <th class=CELL>{move || t("说明")}</th>
                </tr>
              </thead>
              <tbody>
                {DIY_ANTENNAS
                  .iter()
                  .map(|&(name, material, size, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{material}</td>
                        <td class=format!("{CELL} font-mono tabular-nums")>{size}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
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
