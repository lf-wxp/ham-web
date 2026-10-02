//! 传播预测与最佳工作频率速查。

mod voacap_card;

use ham_web_core::muf::{BAND_CHOICE, MUF_CONCEPTS};
use leptos::prelude::*;

use crate::util::set_title;

use crate::i18n::t;
use voacap_card::VoacapCard;

#[component]
pub fn MufPage() -> impl IntoView {
  set_title(&t("传播预测"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("传播预测")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("MUF 最高可用频率 · LUF 最低可用频率 · 最佳工作频率 · 波段选择")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <VoacapCard />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("核心概念")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {MUF_CONCEPTS
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("波段选择建议")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {BAND_CHOICE
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
