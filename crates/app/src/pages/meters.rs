//! 测量仪表速查。

use ham_web_core::meters::METERS;
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn MetersPage() -> impl IntoView {
  set_title(&t("测量仪表"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("测量仪表")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("万用表 · 驻波表 · 功率计 · 天线分析仪")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常用仪表")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("仪表")}</th>
                  <th class=CELL>{move || t("测量对象")}</th>
                  <th class=CELL>{move || t("用途")}</th>
                </tr>
              </thead>
              <tbody>
                {METERS
                  .iter()
                  .map(|&(name, target, usage)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{target}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{usage}</td>
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
