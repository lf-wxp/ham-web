use ham_web_core::rst::{READABILITY, RST_EXAMPLES, SIGNAL_STRENGTH, TONE};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_tone;
use crate::util::set_title;

use super::label_list::LabelList;
use super::rst_listen::RstListen;
use super::rst_quiz::RstQuiz;
use crate::i18n::{t, tf};
use crate::ui::{Button, Size, Variant};

/// 试听音调（Hz）。
const TONE_FREQ: f32 = 700.0;

#[component]
pub fn RstPage() -> impl IntoView {
  set_title("shell.rst-report");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.rst-report")
        subtitle=move || t("knowledge.readability-r-strength-s")
      />

      <PageContainer>
        <div class="grid grid-cols-3 gap-3">
          <div class="pxl-card border bg-card p-4 text-center">
            <div class="pxl-stat">"1–5"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.readability-r")}</div>
          </div>
          <div class="pxl-card border bg-card p-4 text-center">
            <div class="pxl-stat">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.signal-strength-s")}</div>
          </div>
          <div class="pxl-card border bg-card p-4 text-center">
            <div class="pxl-stat">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.tone-t-cw")}</div>
          </div>
        </div>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.r-readability")}</h2>
          <LabelList rows=READABILITY />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">
            {move || t("knowledge.s-strength")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("knowledge.tap-to-play-the")}</span>
          </h2>
          <div class="p-3">
            {SIGNAL_STRENGTH
              .iter()
              .enumerate()
              .map(|(i, &(k, v))| {
                let level = 0.05 + 0.10 * i as f32;
                let pct = (i + 1) * 100 / SIGNAL_STRENGTH.len();
                view! {
                  <div class="flex items-center gap-3 px-1 py-2 transition-colors hover:bg-inset">
                    <Button
                      variant=Variant::Outline
                      size=Size::IconSm
                      class="shrink-0 text-primary"
                      aria_label=Signal::derive(move || tf("common.preview", &[(k)]))
                      on_click=Callback::new(move |_| play_tone(TONE_FREQ, level, 0.7))
                    >
                      <Icon kind=IconKind::Play class="size-6 w-3.5" />
                    </Button>
                    <span class="w-10 shrink-0 font-mono text-sm">{move || kt_tracked(k)}</span>
                    <div class="h-2 flex-1 overflow-hidden bg-muted">
                      <div
                        class="h-full bg-primary transition"
                        style=format!("width: {pct}%")
                      ></div>
                    </div>
                    <span class="w-24 shrink-0 text-sm text-muted-foreground">{move || kt_tracked(v)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.t-tone-cw-only")}</h2>
          <LabelList rows=TONE />
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("knowledge.common-reports")}</h2>
          <LabelList rows=RST_EXAMPLES />
        </section>

        <RstListen />

        <RstQuiz />
      </PageContainer>
    </div>
  }
}
