//! SDR 硬件与 GNU Radio 速查。

use ham_web_core::gnuradio::{GNU_RADIO_CONCEPTS, GNU_RADIO_TIPS, SDR_HARDWARE};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn GnuradioPage() -> impl IntoView {
  set_title("SDR 硬件与 GNU Radio");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"SDR 硬件与 GNU Radio"</div>
            <div class="text-xs text-muted-foreground">"接收机选型 · 软件无线电流图"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"硬件选型"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[520px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"硬件"</th>
                  <th class=CELL>"类型"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {SDR_HARDWARE
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"GNU Radio 概念"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {GNU_RADIO_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"上手要点"</h2>
          <ul class="space-y-2 p-4">
            {GNU_RADIO_TIPS
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
