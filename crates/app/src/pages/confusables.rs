//! 易混淆概念辨析：考试高频易混点速查 + 判断题主动回忆练习。

use ham_web_core::confusables::{CONFUSABLE_QUIZ, CONFUSABLES};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class};
use crate::util::set_title;

/// 辨析判断题练习。
#[component]
fn ConfusablesQuiz() -> impl IntoView {
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
              <div class="text-lg font-semibold">{move || t("练习完成")}</div>
              <div class="mt-2 text-sm text-muted-foreground">
                {move || t("正确")} " " <span class="font-semibold text-emerald-600">{c}</span>
                "　" {move || t("错误")} " " <span class="font-semibold text-red-600">{w}</span>
                "　" {tf("共 {} 题", &[&total.to_string()])}
              </div>
              <button
                type="button"
                class=format!("{} mt-5", button_class(Variant::Default, Size::Default, ""))
                on:click=move |_| restart()
              >
                {move || t("再来一轮")}
              </button>
            </div>
          }
          .into_any()
        } else {
          let item = CONFUSABLE_QUIZ[index.get()];
          view! {
            <div class="rounded-xl border bg-card p-5">
              <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
                <span>{tf("第 {} / {} 题", &[&(index.get() + 1).to_string(), &total.to_string()])}</span>
                <span>
                  {move || t("正确")} " " <span class="font-semibold text-emerald-600">{correct.get()}</span>
                  "　" {move || t("错误")} " " <span class="font-semibold text-red-600">{wrong.get()}</span>
                </span>
              </div>
              <p class="min-h-16 text-base font-medium leading-relaxed">{item.statement}</p>
              <div class="mt-4 flex items-center gap-2">
                <button
                  type="button"
                  class=button_class(Variant::Default, Size::Default, "")
                  disabled=move || answered.get().is_some()
                  on:click=move |_| answer(true)
                >
                  {move || t("正确 ✓")}
                </button>
                <button
                  type="button"
                  class=button_class(Variant::Outline, Size::Default, "")
                  disabled=move || answered.get().is_some()
                  on:click=move |_| answer(false)
                >
                  {move || t("错误 ✗")}
                </button>
                <button
                  type="button"
                  class=button_class(Variant::Ghost, Size::Default, "ml-auto text-muted-foreground")
                  disabled=move || answered.get().is_none()
                  on:click=move |_| next()
                >
                  {move || t("下一题")}
                </button>
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
                        {if is_right { t("回答正确！") } else { t("回答错误。") }}
                        <span class="ml-2 font-normal text-muted-foreground">
                          {format!("{} {}", t("正确答案："), if item.answer { t("正确") } else { t("错误") })}
                        </span>
                      </div>
                      <div class="text-muted-foreground">{item.explain}</div>
                      <div class="text-xs text-primary">
                        {format!("{} {}", t("关联易混点："), t(item.topic))}
                        <a href="#confusables-list" class="ml-1 underline underline-offset-4 hover:underline">
                          {move || t("回看辨析表 →")}
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

#[component]
pub fn ConfusablesPage() -> impl IntoView {
  set_title(&t("易混淆辨析"));
  let quiz = RwSignal::new(false);
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("易混淆概念辨析")}</h1>
            <div class="text-xs text-muted-foreground">
              {move || if quiz.get() { t("主动回忆：判断正误，答错回看辨析表") } else { t("考试里最容易记混的概念，一表看清差别") }}
            </div>
          </div>
          <div class="flex overflow-hidden rounded-lg border text-xs">
            <button
              type="button"
              class=move || if !quiz.get() {
                "px-3 py-1.5 font-medium bg-primary text-primary-foreground"
              } else {
                "px-3 py-1.5 font-medium hover:bg-accent"
              }
              on:click=move |_| quiz.set(false)
            >
              {move || t("速查")}
            </button>
            <button
              type="button"
              class=move || if quiz.get() {
                "px-3 py-1.5 font-medium bg-primary text-primary-foreground"
              } else {
                "px-3 py-1.5 font-medium hover:bg-accent"
              }
              on:click=move |_| quiz.set(true)
            >
              {move || t("辨析练习")}
            </button>
          </div>
        </div>
      </header>

      {move || {
        if quiz.get() {
          view! { <ConfusablesQuiz /> }.into_any()
        } else {
          view! {
            <div id="confusables-list" class="mx-auto max-w-5xl space-y-5 scroll-mt-24 px-4 py-5">
              {CONFUSABLES
                .iter()
                .map(|c| {
                  view! {
                    <section class="rounded-xl border bg-card">
                      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b px-4 py-3">
                        <h2 class="text-sm font-semibold">{move || t(c.title)}</h2>
                        <p class="text-xs text-muted-foreground">{move || t(c.confusion)}</p>
                      </div>
                      <dl class="divide-y">
                        {c
                          .items
                          .iter()
                          .map(|&(name, desc)| {
                            view! {
                              <div class="flex gap-3 px-4 py-2.5">
                                <dt class="w-28 shrink-0 text-sm font-medium text-primary sm:w-36">{move || t(name)}</dt>
                                <dd class="text-sm text-muted-foreground">{move || t(desc)}</dd>
                              </div>
                            }
                          })
                          .collect_view()}
                      </dl>
                      <div class="flex items-start gap-2 border-t bg-muted/40 px-4 py-3">
                        <span class="mt-0.5 shrink-0 text-sm font-semibold text-foreground">{move || t("一句话记住")}</span>
                        <p class="text-sm font-medium">{move || t(c.tip)}</p>
                      </div>
                    </section>
                  }
                })
                .collect_view()}
            </div>
          }
          .into_any()
        }
      }}
    </div>
  }
}
