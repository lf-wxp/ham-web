//! 易混淆概念辨析页：速查表 + 判断题练习的切换外壳。

use ham_web_core::confusables::CONFUSABLES;
use leptos::prelude::*;

use super::confusables_quiz::ConfusablesQuiz;
use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn ConfusablesPage() -> impl IntoView {
  set_title("exam.confusing-concepts");
  let quiz = RwSignal::new(false);
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("exam.confusing-concepts-explained")
        subtitle=move || if quiz.get() { t("exam.active-recall-judge-true") } else { t("exam.the-most-easily-mixed") }
        actions=ViewFn::from(move || {
          view! {
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
                {move || t("exam.reference")}
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
                {move || t("exam.quiz")}
              </button>
            </div>
          }
        })
      />

      {move || {
        if quiz.get() {
          view! { <ConfusablesQuiz /> }.into_any()
        } else {
          view! {
            <PageContainer id="confusables-list" class="space-y-5 scroll-mt-24">
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
                        <span class="mt-0.5 shrink-0 text-sm font-semibold text-foreground">{move || t("exam.remember")}</span>
                        <p class="text-sm font-medium">{move || t(c.tip)}</p>
                      </div>
                    </section>
                  }
                })
                .collect_view()}
            </PageContainer>
          }
          .into_any()
        }
      }}
    </div>
  }
}
