//! DX 奖状体系速查。

use ham_web_core::awards::AWARDS;
use leptos::prelude::*;

use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AwardsPage() -> impl IntoView {
  set_title("DX 奖状体系");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"DX 奖状体系"</div>
            <div class="text-xs text-muted-foreground">"DXCC 世纪俱乐部 · WAZ 全部 CQ 分区 · WAS 全部美国州 · IOTA 空中岛屿"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"主要国际奖状"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[720px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"简称"</th>
                  <th class=CELL>"全称"</th>
                  <th class=CELL>"机构"</th>
                  <th class=CELL>"基本规则"</th>
                </tr>
              </thead>
              <tbody>
                {AWARDS
                  .iter()
                  .map(|&(abbr, full, org, rule)| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} whitespace-nowrap font-mono font-semibold text-primary")>
                          {abbr}
                        </td>
                        <td class=format!("{CELL} whitespace-nowrap font-medium")>{full}</td>
                        <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{org}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{rule}</td>
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
