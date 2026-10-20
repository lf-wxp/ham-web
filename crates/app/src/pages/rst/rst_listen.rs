use ham_web_core::rst::SIGNAL_STRENGTH;
use leptos::prelude::*;

use crate::components::quiz_option::QuizOption;
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_tone;
use crate::ui::{Button, Variant};
use crate::util::random;

/// 试听音调（Hz）。
const TONE_FREQ: f32 = 700.0;

/// 信号强度 → 音量（与 RST 页试听一致：S1≈0.05 … S9≈0.85）。
fn level_of(i: usize) -> f32 {
  0.05 + 0.10 * i as f32
}

/// RST 听力训练：播放随机强度的纯音，判断其 S 值（1–9）。
#[component]
pub(super) fn RstListen() -> impl IntoView {
  let target = RwSignal::new(0usize);
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let new_round = move || {
    target.set((random() * SIGNAL_STRENGTH.len() as f64) as usize);
    feedback.set(None);
  };
  new_round();

  let play = move || {
    let level = level_of(target.get());
    play_tone(TONE_FREQ, level, 0.8);
  };

  let judge = move |i: usize| {
    let ok = i == target.get();
    if ok {
      correct.update(|v| *v += 1);
    } else {
      wrong.update(|v| *v += 1);
    }
    feedback.set(Some(ok));
  };

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">
        {move || t("knowledge.listen-to-the-signal")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("knowledge.play-and-judge-the")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("knowledge.tap-to-play-a")}</div>
          <Button on_click=Callback::new(move |_| play())>
            <Icon kind=IconKind::Volume2 class="size-6" />
            {move || t("knowledge.play-signal")}
          </Button>
        </div>
        <div class="grid grid-cols-3 gap-2 sm:grid-cols-9">
          {SIGNAL_STRENGTH
            .iter()
            .enumerate()
            .map(|(i, &(k, _))| {
              view! {
                <QuizOption
                  class="px-2 py-2 font-mono"
                  disabled=Signal::derive(move || feedback.get().is_some())
                  on_click=Callback::new(move |_| judge(i))
                >
                  {k}
                </QuizOption>
              }
            })
            .collect_view()}
        </div>
        <div class="flex flex-col items-center gap-3">
          <div class="min-h-5 text-sm">
            {move || {
              feedback.get().map(|ok| {
                let (k, v) = SIGNAL_STRENGTH[target.get()];
                if ok {
                  view! {
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">{move || t("learning.correct")}</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      {tf("common.correct-answer", &[(k), (v)])}
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>
          <Button variant=Variant::Outline on_click=Callback::new(move |_| new_round())>
            {move || t("exam.next")}
          </Button>
          <div class="text-xs text-muted-foreground">
            {move || t("common.correct-label")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            {move || t("common.wrong-label")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
