//! 电子电路基础页面：常见元件、基本电路单元与关键公式。

use ham_web_core::electronics::{CIRCUITS, COMPONENTS, FORMULAS};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

use super::component_quiz::ComponentQuiz;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ElectronicsPage() -> impl IntoView {
  set_title(&t("电子电路基础"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("电子电路基础")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("常见元件 · 基本电路 · 关键公式")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常见电子元件")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("元件")}</th>
                  <th class=CELL>{move || t("符号 / 单位")}</th>
                  <th class=CELL>{move || t("作用")}</th>
                </tr>
              </thead>
              <tbody>
                {COMPONENTS
                  .iter()
                  .map(|&(name, unit, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap font-mono text-xs text-muted-foreground")>
                          {unit}
                        </td>
                        <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        <ComponentQuiz />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("基本电路单元")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CIRCUITS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("关键公式")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FORMULAS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium text-primary">{t}</span>
                    <span class="font-mono text-sm text-muted-foreground">{d}</span>
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
