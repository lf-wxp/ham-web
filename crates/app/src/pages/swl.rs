//! SWL 短波监听速查。

use ham_web_core::swl::{SWL_BANDS, SWL_TARGETS, SWL_TIPS};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn SwlPage() -> impl IntoView {
  set_title(&t("SWL 短波监听"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("SWL 短波监听")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("短波广播 · 数传解码 · 接收入门")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("可监听内容")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[520px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("名称")}</th>
                  <th class=CELL>{move || t("类别")}</th>
                  <th class=CELL>{move || t("说明")}</th>
                </tr>
              </thead>
              <tbody>
                {SWL_TARGETS
                  .iter()
                  .map(|&(name, kind, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常用短波广播频段")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SWL_BANDS
              .iter()
              .map(|&(band, range)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{band}</span>
                    <span class="text-sm text-muted-foreground">{range}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("入门要点")}</h2>
          <ul class="space-y-2 p-4">
            {SWL_TIPS
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
      </div>
    </div>
  }
}
