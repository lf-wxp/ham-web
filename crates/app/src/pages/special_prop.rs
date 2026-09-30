//! 特殊传播方式速查。

use ham_web_core::special_prop::SPECIAL_MODES;
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn SpecialPropPage() -> impl IntoView {
  set_title("特殊传播方式");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"特殊传播方式"</div>
            <div class="text-xs text-muted-foreground">"EME · 流星余迹 · 极光 · 对流层散射"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"特殊传播方式"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[640px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"方式"</th>
                  <th class=CELL>"原理"</th>
                  <th class=CELL>"特点"</th>
                </tr>
              </thead>
              <tbody>
                {SPECIAL_MODES
                  .iter()
                  .map(|&(name, principle, feature)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{name}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{principle}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{feature}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </div>
  }
}
