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
  set_title(&t("RST 信号报告"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("RST 信号报告")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("可懂度 R · 信号强度 S · 音调 T")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-3 gap-3">
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–5"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("可懂度 R")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("信号强度 S")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">"1–9"</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("音调 T（CW）")}</div>
          </div>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("R 可懂度（Readability）")}</h2>
          <LabelList rows=READABILITY />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("S 信号强度（Strength）")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("点击播放对应强度")}</span>
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
                      aria-label=tf("试听 {}", &[(k)])
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("T 音调（Tone，仅 CW）")}</h2>
          <LabelList rows=TONE />
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("常用报告")}</h2>
          <LabelList rows=RST_EXAMPLES />
        </section>

        <RstListen />

        <RstQuiz />
      </div>
    </div>
  }
}
