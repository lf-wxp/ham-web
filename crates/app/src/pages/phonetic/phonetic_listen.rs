use ham_web_core::exam::shuffle_in_place;
use ham_web_core::phonetic::PHONETIC;
use leptos::prelude::*;

use crate::i18n::t;
use crate::speech::speak_en;
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
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("morse.listen-to-the-spelling")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.play-the-word-and")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("morse.tap-to-play-a")}</div>
          <button
            type="button"
            on:click=move |_| play()
            class="rounded-lg bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
          >
            {move || t("morse.play-word")}
          </button>
        </div>
        <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {move || {
            options
              .get()
              .into_iter()
              .map(|opt| {
                let chosen = opt.clone();
                view! {
                  <button
                    type="button"
                    on:click=move |_| judge(chosen.clone())
                    class="rounded-lg border px-3 py-2.5 text-center font-mono text-lg font-semibold transition-colors hover:bg-accent"
                  >
                    {opt}
                  </button>
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
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">{move || t("learning.correct")}</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      {move || t("exam.correct-answer-2")} <span class="font-mono font-semibold">{answer.get_value()}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>
          <button
            type="button"
            on:click=move |_| load_next()
            class="rounded-lg border px-4 py-2 text-sm font-medium transition-colors hover:bg-accent"
          >
            {move || t("exam.next")}
          </button>
          <div class="text-xs text-muted-foreground">
            {move || t("common.correct-3")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            {move || t("common.wrong-2")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
