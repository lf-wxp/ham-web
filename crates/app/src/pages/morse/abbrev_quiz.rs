use std::time::Duration;

use ham_web_core::cw_op::CW_ABBREVIATIONS;
use leptos::prelude::*;

use super::random_index;
use crate::i18n::t;
use crate::ui::{Button, Variant};

/// CW 缩语速答：给出缩语，从四个含义中选出正确项。
#[component]
pub(super) fn AbbrevQuiz() -> impl IntoView {
  let current = RwSignal::new(0usize);
  let options = RwSignal::new(Vec::<usize>::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let next = move || {
    let ans = random_index(CW_ABBREVIATIONS.len());
    current.set(ans);
    let mut opts = vec![ans];
    let mut guard = 0;
    while opts.len() < 4 && guard < 100 {
      guard += 1;
      let d = random_index(CW_ABBREVIATIONS.len());
      if !opts.contains(&d) {
        opts.push(d);
      }
    }
    for i in (1..opts.len()).rev() {
      let j = random_index(i + 1);
      opts.swap(i, j);
    }
    options.set(opts);
    feedback.set(None);
  };

  next();

  let choose = move |idx: usize| {
    let ok = idx == current.get();
    if ok {
      correct.update(|v| *v += 1);
    } else {
      wrong.update(|v| *v += 1);
    }
    feedback.set(Some(ok));
    set_timeout(next, Duration::from_millis(900));
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("morse.abbreviation-quiz")}</h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("morse.what-does-this-abbreviation")}</div>
          <div class="font-mono text-4xl font-semibold tabular-nums text-primary">
            {move || CW_ABBREVIATIONS[current.get()].0}
          </div>
          <div class="grid w-full max-w-md grid-cols-1 gap-2 sm:grid-cols-2">
            {move || {
              options
                .get()
                .iter()
                .map(|&idx| {
                  let meaning = CW_ABBREVIATIONS[idx].1.to_owned();
                  view! {
                    <button
                      type="button"
                      on:click=move |_| choose(idx)
                      class="inline-flex min-h-10 items-center justify-center rounded-lg border bg-card px-3 py-2 text-sm transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                    >
                      {meaning}
                    </button>
                  }
                })
                .collect_view()
            }}
          </div>
          <div class="min-h-5 text-sm" aria-live="polite">
            {move || {
              feedback.get().map(|ok| {
                if ok {
                  view! {
                    <span class="inline-flex items-center gap-1.5 font-medium text-emerald-600 dark:text-emerald-400 animate-in fade-in zoom-in-95 duration-200">{move || t("learning.correct")}</span>
                  }
                  .into_any()
                } else {
                  let ans = current.get();
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">"答案是 " {CW_ABBREVIATIONS[ans].1}</span>
                  }
                  .into_any()
                }
              })
            }}
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
          <span>
            {move || t("common.correct-3")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
          </span>
          <span>
            {move || t("morse.wrong")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </span>
          <Button
            variant=Variant::Outline
            class="rounded-lg h-10"
            on_click=Callback::new(move |_| next())
          >{move || t("exam.next")}</Button>
        </div>
      </div>
    </section>
  }
}
