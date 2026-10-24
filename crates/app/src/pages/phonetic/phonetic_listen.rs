use ham_web_core::exam::shuffle_in_place;
use ham_web_core::phonetic::PHONETIC;
use leptos::prelude::*;

use crate::components::quiz_option::QuizOption;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::speech::speak_en;
use crate::ui::{Button, Variant};
use crate::util::random;

/// 字母解释法听力测验：朗读一个代表单词，判断它代表哪个字母。
#[component]
pub(super) fn PhoneticListen() -> impl IntoView {
  let answer = StoredValue::new(String::new());
  let options = RwSignal::new(Vec::<String>::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let load_next = move || {
    let ans_idx = (random() * PHONETIC.len() as f64) as usize;
    let ans = PHONETIC[ans_idx];
    answer.set_value(ans.letter.to_owned());
    // 生成 3 个干扰字母
    let mut distractors: Vec<String> = Vec::new();
    let mut guard = 0;
    while distractors.len() < 3 && guard < 200 {
      guard += 1;
      let di = (random() * PHONETIC.len() as f64) as usize;
      let d = PHONETIC[di];
      if di != ans_idx && !distractors.contains(&d.letter.to_owned()) {
        distractors.push(d.letter.to_owned());
      }
    }
    let mut opts = vec![ans.letter.to_owned()];
    opts.extend(distractors);
    let mut rng = random;
    shuffle_in_place(&mut opts, &mut rng);
    options.set(opts);
    feedback.set(None);
  };
  load_next();

  let play = move || speak_en(&answer.get_value());

  let judge = move |chosen: String| {
    let ok = chosen == answer.get_value();
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
        {move || t("morse.listen-to-the-spelling")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.play-the-word-and")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 border bg-inset px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("morse.tap-to-play-a")}</div>
          <Button on_click=Callback::new(move |_| play())>
            <Icon kind=IconKind::Volume2 class="size-6" />
            {move || t("morse.play-word")}
          </Button>
        </div>
        <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {move || {
            options
              .get()
              .into_iter()
              .map(|opt| {
                let chosen = opt.clone();
                view! {
                  <QuizOption
                    class="text-center font-mono text-lg"
                    disabled=Signal::derive(move || feedback.get().is_some())
                    on_click=Callback::new(move |_| judge(chosen.clone()))
                  >
                    {opt}
                  </QuizOption>
                }
              })
              .collect_view()
          }}
        </div>
        <div class="flex flex-col items-center gap-3">
          <div class="min-h-5 text-sm">
            {move || {
              feedback.get().map(|ok| {
                if ok {
                  view! {
                    <span class="text-win-text">{move || t("learning.correct")}</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="text-hp-text">
                      {move || t("exam.correct-answer-prefix")} <span class="font-mono">{answer.get_value()}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>
          <Button variant=Variant::Outline on_click=Callback::new(move |_| load_next())>
            {move || t("exam.next")}
          </Button>
          <div class="text-xs text-muted-foreground">
            {move || t("common.correct-label")} <span class="tabular-nums text-foreground">{move || correct.get()}</span>
            {move || t("common.wrong-label")} <span class="tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
