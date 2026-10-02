//! 软件定义无线电 SDR 速查。

use ham_web_core::sdr::{SDR_CONCEPTS, SDR_SOFTWARE, WEB_SDR};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn SdrPage() -> impl IntoView {
  set_title(&t("软件定义无线电 SDR"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("软件定义无线电 SDR")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("概念 · 架构 · 常用软件")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("核心概念")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SDR_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常用软件")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {SDR_SOFTWARE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("在线收听（无需本地硬件）")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {WEB_SDR
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
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
