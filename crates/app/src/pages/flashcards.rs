//! 闪卡快速刷题：看题 → 心里想答案 → 显示答案 → 自评，适合考前高强度过题。

use ham_web_core::exam::shuffled;
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::t;
use crate::ui::{Button, Size, Variant};
use crate::util::random;
use crate::util::set_title;

fn pill_class(active: bool) -> &'static str {
  if active {
    "inline-flex items-center whitespace-nowrap rounded-full border bg-primary text-primary-foreground px-3 py-1 text-xs transition-colors"
  } else {
    "inline-flex items-center whitespace-nowrap rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
  }
}

#[component]
pub fn FlashcardsPage() -> impl IntoView {
  set_title("shell.flashcards");

  let bank = RwSignal::new(Bank::A);
  let questions = RwSignal::new(Vec::<QuestionItem>::new());
  let current = RwSignal::new(0usize);
  let revealed = RwSignal::new(false);
  let known = RwSignal::new(0usize);
  let unknown = RwSignal::new(0usize);
  let skipped = RwSignal::new(0usize);
  let loading = RwSignal::new(true);
  let finished = RwSignal::new(false);
  let generation = StoredValue::new(0u32);

  let load = move || {
    loading.set(true);
    finished.set(false);
    current.set(0);
    revealed.set(false);
    known.set(0);
    unknown.set(0);
    skipped.set(0);
    // 递增代次，丢弃过期请求的结果，避免快速切换题库时旧数据覆盖新数据。
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    spawn_local(async move {
      let b = bank.get_untracked();
      let result = data::load_bank(None, b, false).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      if let Ok(qs) = result {
        let qs = shuffled(&qs, &mut random);
        questions.set(qs);
        crate::study::note_question_start();
      }
      loading.set(false);
    });
  };

  load();

  let reveal = move || revealed.set(true);

  // 前进到下一题（不计分，供「跳过」使用）。
  let advance = move || {
    let total = questions.get_untracked().len();
    if current.get_untracked() + 1 < total {
      current.update(|c| *c += 1);
      revealed.set(false);
      crate::study::note_question_start();
    } else {
      finished.set(true);
    }
  };

  let mark = move |ok: bool| {
    if let Some(q) = questions.with_untracked(|qs| qs.get(current.get_untracked()).cloned()) {
      crate::study::record_self_assess(&q, ok);
    }
    if ok {
      known.update(|c| *c += 1);
    } else {
      unknown.update(|c| *c += 1);
    }
    advance();
  };

  let skip = move || {
    skipped.update(|c| *c += 1);
    advance();
  };

  // 看过答案后右划「会」、左划「不会」；未看答案时左划跳过
  let (swipe_start, swipe_end) = crate::gesture::swipe_handlers(Callback::new(move |s| {
    match (revealed.get_untracked(), s) {
      (true, crate::gesture::Swipe::Right) => mark(true),
      (true, crate::gesture::Swipe::Left) => mark(false),
      (false, crate::gesture::Swipe::Left) => skip(),
      (false, crate::gesture::Swipe::Right) => reveal(),
    }
  }));

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.flashcards")
        subtitle=move || t("exam.quick-review-self-rate")
        actions=ViewFn::from(move || {
          view! {
            <div class="flex items-center gap-1">
              {Bank::ALL
                .iter()
                .map(|&b| {
                  view! {
                    <button
                      type="button"
                      on:click=move |_| {
                        bank.set(b);
                        load();
                      }
                      class=move || pill_class(bank.get() == b)
                    >
                      {b.as_str()} {t("exam.class")}
                    </button>
                  }
                })
                .collect_view()}
            </div>
          }
        })
      />

      <PageContainer class="space-y-4">
        {move || {
          if loading.get() {
            view! { <div class="px-4 py-10 text-center text-sm text-muted-foreground">{move || t("exam.loading-questions-u-2026")}</div> }
              .into_any()
          } else if finished.get() {
            let total = known.get() + unknown.get() + skipped.get();
            view! {
              <div class="rounded-xl border bg-card px-4 py-10 text-center">
                <div class="text-lg font-semibold">{move || t("exam.round-complete")}</div>
                <div class="mt-2 text-sm text-muted-foreground">
                  {t("exam.total")} {total} {move || t("exam.questions-know")} <span class="font-semibold text-emerald-700 dark:text-emerald-400">{known.get()}</span>
                  {move || t("exam.don-t-know")} <span class="font-semibold text-red-700 dark:text-red-400">{unknown.get()}</span>
                  {move || {
                    (skipped.get() > 0).then(|| {
                      view! { <span>{move || t("exam.skipped")} <span class="font-semibold text-muted-foreground">{skipped.get()}</span></span> }
                    })
                  }}
                </div>
                <Button
                  variant=Variant::Default
                  size=Size::Default
                  class="mt-5"
                  on_click=Callback::new(move |_| load())
                >
                  {move || t("exam.another-round")}
                </Button>
              </div>
            }
            .into_any()
          } else {
            let Some(q) = questions.get_untracked().get(current.get()).cloned() else {
              return view! { <div></div> }.into_any();
            };
            let total = questions.get_untracked().len();
            view! {
              <div on:touchstart=swipe_start on:touchend=swipe_end class="rounded-xl border bg-card p-5">
                <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
                  <span>{move || t("exam.no")} <span class="font-semibold text-foreground">{current.get() + 1}</span> {" / "} {total} {move || t("exam.questions")}</span>
                  <span>{move || t("exam.know")} <span class="font-semibold text-emerald-700 dark:text-emerald-400">{known.get()}</span> {move || t("exam.don-t-know")} <span class="font-semibold text-red-700 dark:text-red-400">{unknown.get()}</span></span>
                </div>
                <p class="whitespace-pre-line text-base font-medium leading-relaxed">{q.question.clone()}</p>

                {move || {
                  if revealed.get() {
                    view! {
                      <div class="mt-4 space-y-1 border-t pt-3">
                        {q.options.iter().map(|o| {
                          let correct = q.answer_keys.contains(&o.key);
                          view! {
                            <div class=format!(
                              "text-sm {}",
                              if correct { "font-medium text-emerald-600 dark:text-emerald-400" } else { "text-muted-foreground" },
                            )>
                              <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
                            </div>
                          }
                        }).collect_view()}
                      </div>
                    }
                    .into_any()
                  } else {
                    view! { <div class="mt-4 text-xs text-muted-foreground">{move || t("exam.think-of-the-answer")}</div> }
                      .into_any()
                  }
                }}

                <div class="mt-5 flex items-center gap-2">
                  {move || {
                    if revealed.get() {
                      view! {
                        <Button
                          variant=Variant::Default
                          size=Size::Default
                          on_click=Callback::new(move |_| mark(true))
                        >
                          {move || t("learning.know-it")}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Default
                          on_click=Callback::new(move |_| mark(false))
                        >
                          {move || t("learning.don-t-know-2")}
                        </Button>
                      }
                      .into_any()
                    } else {
                      view! {
                        <Button
                          variant=Variant::Default
                          size=Size::Default
                          on_click=Callback::new(move |_| reveal())
                        >
                          {move || t("learning.show-answer")}
                        </Button>
                        <Button
                          variant=Variant::Outline
                          size=Size::Default
                          on_click=Callback::new(move |_| skip())
                        >
                          {move || t("exam.skip")}
                        </Button>
                      }
                      .into_any()
                    }
                  }}
                </div>
              </div>
            }
            .into_any()
          }
        }}
      </PageContainer>
    </div>
  }
}
