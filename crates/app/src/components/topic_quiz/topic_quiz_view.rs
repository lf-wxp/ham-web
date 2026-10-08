//! 专题自测主组件：按当前路由抽取 3 道相关题目并渲染。

use ham_web_core::categories::sub_codes_of;
use ham_web_core::exam::shuffle_in_place;
use ham_web_core::topic_quiz::page_top;
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_location;

use crate::data;
use crate::i18n::t;
use crate::util::random;

use super::quiz_question::QuizQuestion;

#[component]
pub fn TopicQuiz() -> impl IntoView {
  let location = use_location();
  let questions = RwSignal::new(Vec::<QuestionItem>::new());

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
        matched.truncate(3);
        questions.set(matched);
      }
    });
  });

  view! {
    {move || {
      let qs = questions.get();
      if qs.is_empty() {
        return view! { <div></div> }.into_any();
      }
      view! {
        <div class="mx-auto max-w-5xl px-4 pb-10">
          <section class="rounded-xl border bg-card">
            <h2 class="border-b px-4 py-3 text-sm font-semibold">
              {move || t("common.quick-3-question-test")}
              <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("common.from-related-topics-in")}</span>
            </h2>
            <div class="space-y-4 p-4">
              {qs.into_iter().map(|q| view! { <QuizQuestion q=q /> }).collect_view()}
            </div>
          </section>
        </div>
      }
      .into_any()
    }}
  }
}
