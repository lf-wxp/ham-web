//! 业余无线电数字语音与数据模式速查。

use ham_web_core::modes::{DIGITAL_MODES, DIGITAL_USAGE};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ModesPage() -> impl IntoView {
  set_title("数字模式");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"数字模式"</div>
            <div class="text-xs text-muted-foreground">"数字语音与数据模式 · 带宽 · 用途"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl px-4 py-5">
        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[720px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>"模式"</th>
                <th class=CELL>"类型"</th>
                <th class=CELL>"带宽"</th>
                <th class=CELL>"说明"</th>
              </tr>
            </thead>
            <tbody>
              {DIGITAL_MODES
                .iter()
                .map(|m| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap")>
                        <div class="font-medium">{m.name}</div>
                        <div class="font-mono text-xs text-muted-foreground">{m.abbr}</div>
                      </td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{m.kind}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{m.bandwidth}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{m.desc}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="mt-6 rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"按用途选模式"</h2>
          <dl class="divide-y">
            {DIGITAL_USAGE
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[12rem_1fr]">
                    <dt class="font-medium">{k}</dt>
                    <dd class="text-sm text-muted-foreground">{v}</dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>
      </div>
    </div>
  }
}
