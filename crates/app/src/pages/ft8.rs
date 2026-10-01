//! FT8 / FT4 弱信号数字模式操作速查。

use ham_web_core::ft8::{FT8_CONCEPTS, FT8_FREQS, FT8_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

#[component]
pub fn Ft8Page() -> impl IntoView {
  set_title("FT8 / FT4");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"FT8 / FT4"</h1>
            <div class="text-xs text-muted-foreground">"弱信号数字模式（FT8 = 8-FSK / FT4 = 4-FSK）· WSJT-X 操作 · 标准频率"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"核心概念"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FT8_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"标准频率（USB 模式）"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FT8_FREQS
              .iter()
              .map(|&(freq, band)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="w-28 shrink-0 font-mono text-sm font-semibold text-primary">{freq}</span>
                    <span class="text-sm text-muted-foreground">{band}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"操作要点"</h2>
          <ul class="space-y-2 p-4">
            {FT8_TIPS
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
