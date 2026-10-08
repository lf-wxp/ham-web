use ham_web_core::rst::{READABILITY, RST_EXAMPLES, SIGNAL_STRENGTH, TONE};
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_tone;
use crate::util::set_title;

use super::label_list::LabelList;
use super::rst_listen::RstListen;
use super::rst_quiz::RstQuiz;
use crate::i18n::{t, tf};

/// 试听音调（Hz）。
const TONE_FREQ: f32 = 700.0;

#[component]
pub fn RstPage() -> impl IntoView {
  set_title("shell.rst-report");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.rst-report")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("knowledge.readability-r-strength-s")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-3 gap-3">
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–5"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.readability-r")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.signal-strength-s")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("knowledge.tone-t-cw")}</div>
          </div>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.r-readability")}</h2>
          <LabelList rows=READABILITY />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
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
                  <div class="flex items-center gap-3 rounded-lg px-1 py-2 transition-colors hover:bg-muted/40">
                    <button
                      type="button"
                      aria-label=tf("common.preview", &[(k)])
                      on:click=move |_| play_tone(TONE_FREQ, level, 0.7)
                      class="flex size-8 shrink-0 items-center justify-center rounded-full border text-primary transition-colors hover:bg-accent"
                    >
                      <Icon kind=IconKind::Play class="h-3.5 w-3.5" />
                    </button>
                    <span class="w-10 shrink-0 font-mono text-sm font-semibold">{k}</span>
                    <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                      <div
                        class="h-full rounded-full bg-primary transition-all"
                        style=format!("width: {pct}%")
                      ></div>
                    </div>
                    <span class="w-24 shrink-0 text-sm text-muted-foreground">{v}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.t-tone-cw-only")}</h2>
          <LabelList rows=TONE />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.common-reports")}</h2>
          <LabelList rows=RST_EXAMPLES />
        </section>

        <RstListen />

        <RstQuiz />
      </div>
    </div>
  }
}
