use ham_web_core::exam::shuffle_in_place;
use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;

use crate::i18n::t;
use crate::pages::SLANG_CATEGORY;
use crate::util::random;

/// 常用 Q 简语词条（三字母、以 Q 开头、ASCII、常用）。
fn qcode_pool(entries: &[GlossaryEntry]) -> Vec<&GlossaryEntry> {
  entries
    .iter()
    .filter(|e| {
      e.category_key() == SLANG_CATEGORY
        && e.term.len() == 3
        && e.term.starts_with('Q')
        && e.term.is_ascii()
        && e.common
    })
    .collect()
}

/// Q 简语反向测验：给出含义，从四个简语中选出正确项。
#[component]
pub(super) fn QCodeReverseQuiz(entries: &'static [GlossaryEntry]) -> impl IntoView {
  let question = RwSignal::new(String::new());
  let options = RwSignal::new(Vec::<String>::new());
  let answer = StoredValue::new(String::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let load_next = move || {
    let pool = qcode_pool(entries);
    if pool.len() < 4 {
      return;
    }
    let ans_idx = (random() * pool.len() as f64) as usize;
    let ans = pool[ans_idx];
    answer.set_value(ans.term.clone());
    question.set(ans.desc.clone());
    let mut distractors: Vec<String> = Vec::new();
    let mut guard = 0;
    while distractors.len() < 3 && guard < 200 {
      guard += 1;
      let di = (random() * pool.len() as f64) as usize;
      let d = pool[di];
      if di != ans_idx && !distractors.contains(&d.term) {
        distractors.push(d.term.clone());
      }
    }
    let mut opts = vec![ans.term.clone()];
    opts.extend(distractors);
    let mut rng = random;
    shuffle_in_place(&mut opts, &mut rng);
    options.set(opts);
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
    <section class="mb-8 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("Q 简语反向测验")}</h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("这个含义对应哪个 Q 简语？")}</div>
          <div class="text-base font-semibold text-foreground">{move || question.get()}</div>
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
                    class="rounded-lg border px-3 py-2.5 text-center font-mono text-sm font-semibold transition-colors hover:bg-accent"
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
                      {move || t("正确答案：")} <span class="font-mono font-semibold">{answer.get_value()}</span>
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
            {move || t("下一题")}
          </button>
          <div class="text-xs text-muted-foreground">
            {move || t("正确 ")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            {move || t("　错误 ")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </div>
        </div>
      </div>
    </section>
  }
}
