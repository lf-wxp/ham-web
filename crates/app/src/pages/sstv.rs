//! SSTV 慢扫描电视：静止图像传输速查。

use ham_web_core::sstv::{SSTV_CONCEPTS, SSTV_FREQS, SSTV_MODES, SSTV_TIPS};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn SstvPage() -> impl IntoView {
  set_title("shell.sstv");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.sstv")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.still-images-within-a")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.key-concepts-2")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SSTV_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("shell.common-frequencies")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2 md:grid-cols-3">
            {SSTV_FREQS
              .iter()
              .map(|&(b, f)| {
                view! {
                  <div class="rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{b}</span>
                    <span class="ml-2 font-mono text-sm text-muted-foreground">{f}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[520px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("log.mode")}</th>
                <th class=CELL>{move || t("knowledge.resolution-duration")}</th>
                <th class=CELL>{move || t("radio.notes")}</th>
              </tr>
            </thead>
            <tbody>
              {SSTV_MODES
                .iter()
                .map(|&(name, res, desc)| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{res}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{desc}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {SSTV_TIPS
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
