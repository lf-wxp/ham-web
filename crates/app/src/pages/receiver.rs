//! 接收机关键指标详解。

use ham_web_core::receiver::{NOISE_BASICS, RECEIVER_METRICS, RECEIVER_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ReceiverPage() -> impl IntoView {
  set_title("接收机关键指标");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"接收机关键指标"</div>
            <div class="text-xs text-muted-foreground">"灵敏度 · 选择性 · 动态范围 · 三阶截点"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[640px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>"指标"</th>
                <th class=CELL>"含义"</th>
                <th class=CELL>"决定因素"</th>
              </tr>
            </thead>
            <tbody>
              {RECEIVER_METRICS
                .iter()
                .map(|&(name, meaning, factor)| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{meaning}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{factor}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"噪声与灵敏度基础"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {NOISE_BASICS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"改善接收的要点"</h2>
          <ul class="space-y-2 p-4">
            {RECEIVER_TIPS
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
