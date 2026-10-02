//! 新手入门指南速查。

use ham_web_core::beginner::{BEGINNER_STEPS, FIRST_QSO_TIPS, RIG_TYPES};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn BeginnerPage() -> impl IntoView {
  set_title(&t("新手入门指南"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("新手入门指南")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("从考证到首次通联 · 设备选购")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("入门路线图")}</h2>
          <div class="space-y-1 p-4">
            {BEGINNER_STEPS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2 sm:flex-row sm:items-baseline sm:gap-3">
                    <span class="shrink-0 text-sm font-semibold text-primary">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("设备类型")}</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("类型")}</th>
                  <th class=CELL>{move || t("频段")}</th>
                  <th class=CELL>{move || t("说明")}</th>
                </tr>
              </thead>
              <tbody>
                {RIG_TYPES
                  .iter()
                  .map(|&(name, band, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} whitespace-nowrap font-mono text-muted-foreground")>
                          {band}
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("首次通联要点")}</h2>
          <ul class="space-y-2 p-4">
            {FIRST_QSO_TIPS
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
