//! FT8 / FT4 弱信号数字模式操作速查。

use ham_web_core::ft8::{FT8_CONCEPTS, FT8_FREQS, FT8_TIPS};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn Ft8Page() -> impl IntoView {
  set_title("FT8 / FT4");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title="FT8 / FT4".to_string()
        subtitle=move || t("knowledge.weak-signal-digital-modes")
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.key-concepts-heading")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FT8_CONCEPTS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.standard-frequencies-usb")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {FT8_FREQS
              .iter()
              .map(|&(freq, band)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="w-28 shrink-0 font-mono text-sm text-primary">{move || kt_tracked(freq)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(band)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {FT8_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </PageContainer>
    </div>
  }
}
