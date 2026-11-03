//! 专题自测主组件：按当前路由抽取 3 道相关题目并渲染。

use ham_web_core::categories::sub_codes_of;
use ham_web_core::exam::shuffle_in_place;
use ham_web_core::topic_quiz::page_top;
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_location;

use crate::components::common::PageContainer;
use crate::data;
use crate::i18n::t;
use crate::util::random;

use super::quiz_question::QuizQuestion;

#[component]
pub fn TopicQuiz() -> impl IntoView {
  let location = use_location();
  let questions = RwSignal::new(Vec::<QuestionItem>::new());
  // 抽中的全量候选：点「换一组」时从这里重新抽，不用再读一次题库。
  let pool = StoredValue::new(Vec::<QuestionItem>::new());
  // 每题判分结果（None = 还没答）。用于顶部「已完成 / 正确」计数。
  let verdicts = RwSignal::new(Vec::<Option<bool>>::new());

  Effect::new(move |_| {
    let route = location.pathname.get();
    let Some(top) = page_top(&route) else {
      questions.set(Vec::new());
      return;
    };
    let codes = sub_codes_of(top);
    if codes.is_empty() {
      questions.set(Vec::new());
      return;
    }
    let codes: Vec<&'static str> = codes;
    spawn_local(async move {
      if let Ok(qs) = data::load_bank(None, Bank::A, false).await {
        let mut matched: Vec<QuestionItem> = qs
          .iter()
          .filter(|q| q.p_code().is_some_and(|p| codes.contains(&p)))
          .cloned()
          .collect();
        let mut rng = random;
        shuffle_in_place(&mut matched, &mut rng);
        pool.set_value(matched.clone());
        matched.truncate(3);
        verdicts.set(vec![None; matched.len()]);
        questions.set(matched);
      }
    });
  });

  let reshuffle = Callback::new(move |()| {
    pool.with_value(|all| {
      if all.is_empty() {
        return;
      }
      let mut next = all.clone();
      let mut rng = random;
      shuffle_in_place(&mut next, &mut rng);
      next.truncate(3);
      verdicts.set(vec![None; next.len()]);
      questions.set(next);
    });
  });

  view! {
    {move || {
      let qs = questions.get();
      if qs.is_empty() {
        return view! { <div></div> }.into_any();
      }
      if crate::kit::is_modern() {
        let total = qs.len();
        let done = move || verdicts.with(|v| v.iter().filter(|x| x.is_some()).count());
        let right = move || verdicts.with(|v| v.iter().filter(|x| **x == Some(true)).count());
        return view! {
          <div class="md-page md-related">
            <section class="md-quiz-panel">
              <header class="md-quiz-bar">
                <div class="md-quiz-bar-main">
                  <h2 class="md-quiz-title">{move || t("common.quick-3-question-test")}</h2>
                  <p class="md-quiz-sub">{move || t("common.from-related-topics-in")}</p>
                </div>
                <div class="md-quiz-score" aria-live="polite">
                  <span class="md-mono">
                    {move || format!("{}/{}", done(), total)}
                  </span>
                  <span class="md-quiz-score-label">
                    {move || t("exam.correct-label")} " " {move || right().to_string()}
                  </span>
                </div>
              </header>

              <ol class="md-quiz-list">
                {qs
                  .into_iter()
                  .enumerate()
                  .map(|(i, q)| {
                    view! {
                      <li>
                        <QuizQuestion
                          q=q
                          index=i + 1
                          on_verdict=Callback::new(move |ok: bool| {
                            verdicts.update(|v| {
                              if let Some(slot) = v.get_mut(i) {
                                *slot = Some(ok);
                              }
                            });
                          })
                        />
                      </li>
                    }
                  })
                  .collect_view()}
              </ol>

              <footer class="md-quiz-foot">
                {move || {
                  (total > 0 && done() == total)
                    .then(|| {
                      view! {
                        <p class="md-quiz-summary" data-all=move || (right() == total).to_string()>
                          {move || t("common.quiz-summary")}
                        </p>
                      }
                    })
                }}
                <button type="button" class="md-btn md-btn-quiet" on:click=move |_| reshuffle.run(())>
                  {move || t("common.quiz-retry")}
                </button>
              </footer>
            </section>
          </div>
        }
        .into_any();
      }
      view! {
        <PageContainer class="pt-0 pb-10">
          <section class=crate::kit::card_class()>
            <h2 class=crate::kit::card_title_class()>
              {move || t("common.quick-3-question-test")}
              <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("common.from-related-topics-in")}</span>
            </h2>
            <div class="space-y-4 p-4">
              {qs
                .into_iter()
                .enumerate()
                .map(|(i, q)| view! { <QuizQuestion q=q index=i + 1 /> })
                .collect_view()}
            </div>
          </section>
        </PageContainer>
      }
      .into_any()
    }}
  }
}
