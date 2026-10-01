//! 户外便携操作速查：SOTA、POTA 等。

use ham_web_core::portable::{PORTABLE_PROGRAMS, PORTABLE_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn PortablePage() -> impl IntoView {
  set_title("户外便携操作");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"户外便携操作"</h1>
            <div class="text-xs text-muted-foreground">"SOTA 山顶 · POTA 公园 · WWFF 世界动植物"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"主要项目"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"项目"</th>
                  <th class=CELL>"全称"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {PORTABLE_PROGRAMS
                  .iter()
                  .map(|&(name, full, desc)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>{name}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{full}</td>
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"便携要点"</h2>
          <ul class="space-y-2 p-4">
            {PORTABLE_TIPS
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
