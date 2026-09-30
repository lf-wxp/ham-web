use ham_web_core::antenna_modeling::{MODELING_SOFTWARE, MODELING_STEPS, MODELING_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

use super::dipole_calculator::DipoleCalculator;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn AntennaModelingPage() -> impl IntoView {
  set_title("天线建模软件");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"天线建模软件"</div>
            <div class="text-xs text-muted-foreground">"EZNEC · MMANA-GAL · 4NEC2"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"偶极天线尺寸估算"</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            "先估算尺寸再建模：半波偶极总长 ≈ 150/f × k（单臂为总长一半），建议架高约半波长。"
          </p>
          <div class="p-4"><DipoleCalculator /></div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"常用软件"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[520px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"软件"</th>
                  <th class=CELL>"类型"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {MODELING_SOFTWARE
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"建模流程"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MODELING_STEPS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"使用要点"</h2>
          <ul class="space-y-2 p-4">
            {MODELING_TIPS
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
