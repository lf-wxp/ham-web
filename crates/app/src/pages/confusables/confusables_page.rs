//! 易混淆概念辨析页：速查表 + 判断题练习的切换外壳。

use ham_web_core::confusables::CONFUSABLES;
use leptos::prelude::*;

use super::confusables_quiz::ConfusablesQuiz;
use crate::i18n::t;
use crate::util::set_title;

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
