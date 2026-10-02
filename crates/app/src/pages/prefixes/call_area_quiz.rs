use ham_web_core::exam::shuffle_in_place;
use ham_web_core::reference::CALL_AREAS;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::util::random;

/// 呼号分区测验：给分区号（0–9）选对应地区。
#[component]
pub(super) fn CallAreaQuiz() -> impl IntoView {
  let question = RwSignal::new(String::new());
  let options = RwSignal::new(Vec::<String>::new());
  let answer = StoredValue::new(String::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let load_next = move || {
    if CALL_AREAS.len() >= 4 {
      let ans_idx = (random() * CALL_AREAS.len() as f64) as usize;
      let ans = &CALL_AREAS[ans_idx];
      let mut distractors: Vec<String> = Vec::new();
      let mut guard = 0;
      while distractors.len() < 3 && guard < 200 {
        guard += 1;
        let di = (random() * CALL_AREAS.len() as f64) as usize;
        let d = &CALL_AREAS[di];
        if di != ans_idx && d.regions != ans.regions && !distractors.contains(&d.regions.to_owned())
        {
          distractors.push(d.regions.to_owned());
        }
      }
      let mut opts = vec![ans.regions.to_owned()];
      opts.extend(distractors);
      let mut rng = random;
      shuffle_in_place(&mut opts, &mut rng);
      question.set(ans.digit.to_owned());
      options.set(opts);
      answer.set_value(ans.regions.to_owned());
    }
    feedback.set(None);
  };
  load_next();

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
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("分区测验")}</h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("这个分区号对应的地区是？")}</div>
          <div class="font-mono text-3xl font-semibold text-primary">{move || tf("{} 区", &[&(question.get()).to_string()])}</div>
        </div>
        <div class="grid gap-2 sm:grid-cols-2">
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
                    class="rounded-lg border px-3 py-2.5 text-left text-sm transition-colors hover:bg-accent"
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
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">{move || t("正确！")}</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      {t("正确答案：")} {answer.get_value()}
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
            class="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
          >
            {move || t("下一题")}
          </button>
          <div class="text-xs text-muted-foreground">
            {t("正确 ")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            {t("　错误 ")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
