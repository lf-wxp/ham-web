//! QSL 卡片设计速查。

use ham_web_core::qsl_card::{QSL_DESIGN_TIPS, QSL_REQUIRED};
use leptos::prelude::*;

use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn QslCardPage() -> impl IntoView {
  set_title(&t("QSL 卡片"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("QSL 卡片设计")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("必备信息 · 设计建议")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("必备信息")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {QSL_REQUIRED
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("设计建议")}</h2>
          <ul class="space-y-2 p-4">
            {QSL_DESIGN_TIPS
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
