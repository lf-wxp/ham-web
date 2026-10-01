//! 电子 QSL 与日志确认速查。

use ham_web_core::eqsl::{EQSL_NOTES, EQSL_SERVICES};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn EqslPage() -> impl IntoView {
  set_title("电子 QSL");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"电子 QSL"</h1>
            <div class="text-xs text-muted-foreground">"LoTW 日志确认 · eQSL 电子卡片 · QRZ · Club Log"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"主要服务"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"服务"</th>
                  <th class=CELL>"类型"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {EQSL_SERVICES
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"说明要点"</h2>
          <ul class="space-y-2 p-4">
            {EQSL_NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*note}</span>
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
