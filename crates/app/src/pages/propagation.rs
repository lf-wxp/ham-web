//! 无线电传播与电离层速查：电离层分层、传播方式与关键概念。

use ham_web_core::propagation::{CONCEPTS, LAYERS, LOW_BAND_TIPS, PROPAGATION_MODES};
use leptos::prelude::*;

use crate::util::set_title;

#[component]
pub fn PropagationPage() -> impl IntoView {
  set_title("传播与电离层");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"传播与电离层"</div>
            <div class="text-xs text-muted-foreground">"电离层分层 · 传播方式 · 关键概念"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"电离层分层"</h2>
          <div class="divide-y">
            {LAYERS
              .iter()
              .map(|&(name, range, desc)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[8rem_10rem_1fr]">
                    <div class="font-medium">{name}</div>
                    <div class="font-mono text-xs text-muted-foreground">{range}</div>
                    <div class="text-sm text-muted-foreground">{desc}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"主要传播方式"</h2>
          <dl class="divide-y">
            {PROPAGATION_MODES
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[10rem_1fr]">
                    <dt class="font-medium">{k}</dt>
                    <dd class="text-sm text-muted-foreground">{v}</dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"关键概念"</h2>
          <dl class="divide-y">
            {CONCEPTS
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"低频段（160m / 80m）DX 要点"</h2>
          <ul class="space-y-2 p-4">
            {LOW_BAND_TIPS
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
