//! 易混淆概念辨析的判断题练习。

use ham_web_core::confusables::CONFUSABLE_QUIZ;
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::ui::{Button, Size, Variant};

/// 辨析判断题练习。
#[component]
pub(super) fn ConfusablesQuiz() -> impl IntoView {
  let total = CONFUSABLE_QUIZ.len();
  let index = RwSignal::new(0usize);
  let answered = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);
  let finished = RwSignal::new(false);

  let answer = move |choice: bool| {
    if answered.get_untracked().is_some() {
      return;
    }
    let item = CONFUSABLE_QUIZ[index.get_untracked()];
    answered.set(Some(choice));
    if choice == item.answer {
      correct.update(|c| *c += 1);
    } else {
      wrong.update(|c| *c += 1);
    }
  };
  let next = move || {
    if index.get_untracked() + 1 >= total {
      finished.set(true);
    } else {
      index.update(|i| *i += 1);
      answered.set(None);
    }
  };
  let restart = move || {
    index.set(0);
    answered.set(None);
    correct.set(0);
    wrong.set(0);
    finished.set(false);
  };

  view! {
    <div class="mx-auto max-w-2xl px-4 py-5">
      {move || {
        if finished.get() {
          let c = correct.get();
          let w = wrong.get();
          view! {
            <div class="rounded-xl border bg-card px-4 py-10 text-center">
              <div class="text-lg font-semibold">{move || t("exam.practice-complete")}</div>
              <div class="mt-2 text-sm text-muted-foreground">
                {move || t("exam.correct-3")} " " <span class="font-semibold text-emerald-600">{c}</span>
                "　" {move || t("exam.wrong-2")} " " <span class="font-semibold text-red-600">{w}</span>
                "　" {tp("common.questions-3", total, &[&total.to_string()])}
              </div>
              <Button
                variant=Variant::Default
                size=Size::Default
                class="mt-5"
                on_click=Callback::new(move |_| restart())
              >
                {move || t("exam.another-round")}
              </Button>
            </div>
          }
          .into_any()
        } else {
          let item = CONFUSABLE_QUIZ[index.get()];
          view! {
            <div class="rounded-xl border bg-card p-5">
              <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
                <span>{tf("exam.question", &[&(index.get() + 1).to_string(), &total.to_string()])}</span>
                <span>
                  {move || t("exam.correct-3")} " " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　" {move || t("exam.wrong-2")} " " <span class="font-semibold text-red-600">{wrong.get()}</span>
                </span>
              </div>
              <p class="min-h-16 text-base font-medium leading-relaxed">{item.statement}</p>
              <div class="mt-4 flex items-center gap-2">
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  disabled=Signal::derive(move || answered.get().is_some())
                  on_click=Callback::new(move |_| answer(true))
                >
                  {move || t("exam.true")}
                </Button>
                <Button
                  variant=Variant::Outline
                  size=Size::Default
                  disabled=Signal::derive(move || answered.get().is_some())
                  on_click=Callback::new(move |_| answer(false))
                >
                  {move || t("exam.false")}
                </Button>
                <Button
                  variant=Variant::Ghost
                  size=Size::Default
                  class="ml-auto text-muted-foreground"
                  disabled=Signal::derive(move || answered.get().is_none())
                  on_click=Callback::new(move |_| next())
                >
                  {move || t("exam.next")}
                </Button>
              </div>
              {move || {
                answered.get().map(|choice| {
                  let is_right = choice == item.answer;
                  view! {
                    <div class="mt-4 space-y-1 border-t pt-3 text-sm">
                      <div class=if is_right {
                        "font-medium text-emerald-700 dark:text-emerald-400"
                      } else {
                        "font-medium text-red-700 dark:text-red-400"
                      }>
                        {if is_right { t("exam.correct") } else { t("exam.wrong") }}
                        <span class="ml-2 font-normal text-muted-foreground">
                          {format!("{} {}", t("exam.correct-answer-2"), if item.answer { t("exam.correct-3") } else { t("exam.wrong-2") })}
                        </span>
                      </div>
                      <div class="text-muted-foreground">{item.explain}</div>
                      <div class="text-xs text-primary">
                        {format!("{} {}", t("exam.related-topic"), t(item.topic))}
                        <a href="#confusables-list" class="ml-1 underline underline-offset-4 hover:underline">
                          {move || t("exam.review-table")}
                        </a>
                      </div>
                    </div>
                  }
                })
              }}
            </div>
          }
          .into_any()
        }
      }}
    </div>
  }
}
