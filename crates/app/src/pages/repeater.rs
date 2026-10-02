//! 中继台与数字网关速查。

use ham_web_core::repeater::{DIGITAL_GATEWAYS, INTERNET_GATEWAYS, REPEATER_CONCEPTS};
use leptos::prelude::*;

use super::repeater_lookup::RepeaterLookup;
use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn RepeaterPage() -> impl IntoView {
  set_title(&t("中继台与数字网关"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("中继台与数字网关")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("中继台原理 · 数字中继 · 热点 · 互联网网关")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("中继台概念")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {REPEATER_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("数字中继与网关")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("类型")}</th>
                  <th class=CELL>{move || t("协议")}</th>
                  <th class=CELL>{move || t("说明")}</th>
                </tr>
              </thead>
              <tbody>
                {DIGITAL_GATEWAYS
                  .iter()
                  .map(|&(name, protocol, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{protocol}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <RepeaterLookup />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("互联网网关")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {INTERNET_GATEWAYS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}
