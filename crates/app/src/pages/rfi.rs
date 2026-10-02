//! 射频干扰与电磁兼容（RFI / EMC）速查。

use ham_web_core::rfi::{RFI_SOLUTIONS, RFI_SOURCES, RFI_TIPS};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn RfiPage() -> impl IntoView {
  set_title(&t("射频干扰与电磁兼容"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("射频干扰与电磁兼容")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("RFI · EMC · 干扰排查与抑制")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常见干扰源")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {RFI_SOURCES
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("排查与抑制手段")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {RFI_SOLUTIONS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("处理要点")}</h2>
          <ul class="space-y-2 p-4">
            {RFI_TIPS
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
