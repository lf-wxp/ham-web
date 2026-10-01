//! 模拟通信模式速查：CW、SSB、AM、FM 等传统话音与电报模式。

use ham_web_core::analog_modes::{ANALOG_MODES, ANALOG_VS_DIGITAL, SIDEBAND_RULES};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AnalogModesPage() -> impl IntoView {
  set_title("模拟模式");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"模拟模式"</h1>
            <div class="text-xs text-muted-foreground">"CW 等幅波 · SSB 单边带 · AM 调幅 · FM 调频 · SSTV 慢扫描电视"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[720px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>"模式"</th>
                <th class=CELL>"发射类别"</th>
                <th class=CELL>"带宽"</th>
                <th class=CELL>"说明"</th>
                <th class=CELL>"特点"</th>
                <th class=CELL>"典型用途"</th>
              </tr>
            </thead>
            <tbody>
              {ANALOG_MODES
                .iter()
                .map(|m| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap")>
                        <div class="font-medium">{m.name}</div>
                        <div class="font-mono text-xs text-muted-foreground">{m.abbr}</div>
                      </td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono text-muted-foreground")>{m.emission}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{m.bandwidth}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{m.desc}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{m.pros}</td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{m.usage}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"USB / LSB 边带选择惯例"</h2>
          <ul class="space-y-2 p-4">
            {SIDEBAND_RULES
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"模拟 vs 数字"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ANALOG_VS_DIGITAL
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
      </div>
    </div>
  }
}
