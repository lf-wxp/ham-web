//! Maidenhead 网格定位速查。

use ham_web_core::grid_system::{GRID_LEVELS, GRID_NOTES};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn GridSystemPage() -> impl IntoView {
  set_title("网格定位");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"Maidenhead 网格定位"</h1>
            <div class="text-xs text-muted-foreground">"网格层级 · 精度 · 应用"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"网格层级"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"位数"</th>
                  <th class=CELL>"名称示例"</th>
                  <th class=CELL>"精度"</th>
                </tr>
              </thead>
              <tbody>
                {GRID_LEVELS
                  .iter()
                  .map(|&(bits, example, precision)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{bits}</td>
                        <td class=format!("{CELL} whitespace-nowrap font-mono")>{example}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{precision}</td>
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
            {GRID_NOTES
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
