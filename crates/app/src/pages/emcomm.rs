//! 应急通信 EmComm 速查。

use ham_web_core::emcomm::{EMCOMM_CONCEPTS, EMCOMM_FREQS, EMCOMM_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

#[component]
pub fn EmcommPage() -> impl IntoView {
  set_title("应急通信 EmComm");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"应急通信 EmComm"</div>
            <div class="text-xs text-muted-foreground">"ARES 业余应急服务 · RACES 业余民防 · 应急频率"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"组织与概念"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {EMCOMM_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"应急常用频率"</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {EMCOMM_FREQS
              .iter()
              .map(|&(freq, usage)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 font-mono text-sm font-semibold text-primary">{freq}</span>
                    <span class="text-sm text-muted-foreground">{usage}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"操作要点"</h2>
          <ul class="space-y-2 p-4">
            {EMCOMM_TIPS
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
