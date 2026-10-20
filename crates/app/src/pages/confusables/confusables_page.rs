//! 易混淆概念辨析页：速查表 + 判断题练习的切换外壳。

use ham_web_core::confusables::CONFUSABLES;
use leptos::prelude::*;

use super::confusables_quiz::ConfusablesQuiz;
use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::ui::{Chip, ChipGroup};
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
            <ChipGroup
              value=Signal::derive(move || if quiz.get() { "quiz" } else { "reference" }.to_owned())
              on_change=Callback::new(move |v: String| quiz.set(v == "quiz"))
              aria_label=Signal::derive(move || t("exam.confusing-concepts"))
            >
              <Chip value="reference".to_owned()>
                {view! { {move || t("exam.reference")} }.into_any()}
              </Chip>
              <Chip value="quiz".to_owned()>
                {view! { {move || t("exam.quiz")} }.into_any()}
              </Chip>
            </ChipGroup>
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
                    <section class="pxl-card border bg-card">
                      <h2 class="flex flex-wrap items-baseline gap-x-3 gap-y-1 pxl-section-title">
                        {move || t(c.title)}
                        <span class="text-xs font-normal normal-case text-muted-foreground">{move || t(c.confusion)}</span>
                      </h2>
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
