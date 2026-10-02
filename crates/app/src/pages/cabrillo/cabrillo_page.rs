use ham_web_core::cabrillo::{CABRILLO_CONCEPTS, CABRILLO_TIPS};
use leptos::prelude::*;

use crate::util::set_title;

use super::cabrillo_generator::CabrilloGenerator;
use crate::i18n::t;

#[component]
pub fn CabrilloPage() -> impl IntoView {
  set_title(&t("竞赛 Cabrillo 日志"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("竞赛 Cabrillo 日志")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("标准格式 · 记分 · 提交")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("格式结构")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CABRILLO_CONCEPTS
              .iter()
              .map(|&(title, desc)| {
                view! {
                  <div class="flex flex-col gap-1 rounded-lg px-3 py-2">
                    <span class="text-sm font-medium">{move || t(title)}</span>
                    <span class="text-sm text-muted-foreground">{move || t(desc)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("提交要点")}</h2>
          <ul class="space-y-2 p-4">
            {CABRILLO_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || t(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <CabrilloGenerator />
      </div>
    </div>
  }
}
