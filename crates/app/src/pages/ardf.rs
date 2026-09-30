//! 无线电测向 ARDF 速查。

use ham_web_core::ardf::{ARDF_BANDS, ARDF_CONCEPTS};
use leptos::prelude::*;

use crate::util::set_title;

#[component]
pub fn ArdfPage() -> impl IntoView {
  set_title("无线电测向 ARDF");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"无线电测向 ARDF"</div>
            <div class="text-xs text-muted-foreground">"业余无线电测向（ARDF）· 竞赛频段"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"核心概念"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ARDF_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"常用频段"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {ARDF_BANDS
              .iter()
              .map(|&(band, desc)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{band}</span>
                    <span class="text-sm text-muted-foreground">{desc}</span>
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
