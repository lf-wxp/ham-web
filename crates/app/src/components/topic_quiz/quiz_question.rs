//! 单道自测题：独立管理自己的作答与判分状态。

use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::components::common::ExplanationCard;
use crate::i18n::t;

/// 一道自测题（独立组件，管理自己的作答状态）。
#[component]
pub(super) fn QuizQuestion(q: QuestionItem) -> impl IntoView {
  let selected = RwSignal::new(Vec::<String>::new());
  let is_multiple = q.is_multiple();
  let question = q.question.clone();
  let answer_keys = q.answer_keys.clone();
  let options = q.options.clone();

  let toggle = move |key: String| {
    selected.update(|s| {
      if is_multiple {
        if let Some(pos) = s.iter().position(|k| *k == key) {
          s.remove(pos);
        } else {
          s.push(key);
        }
      } else {
        *s = vec![key];
      }
    });
  };
  let answered = move || !selected.get().is_empty();
  // 解析卡片只关心「有没有作答」这个布尔值：用 Memo 去重，改选项时不会把卡片重建、
  // 把用户刚展开的解析又收起来（对错提示那一行跟着选项重渲染，不受影响）。
  let has_answer = Memo::new(move |_| answered());
  let explain = q.explanation.clone();
  let correct = move || q.is_answer_correct(&selected.get());

  view! {
    <div class="border p-3">
      <div class="text-sm">{question}</div>
      <div class="mt-2 space-y-1.5">
        {options
          .into_iter()
          .map(|o| {
            let key_label = o.key.clone();
            let text = o.text.clone();
            let is_sel = {
              let k = o.key.clone();
              move || selected.with(|s| s.contains(&k))
            };
            let class_key = o.key.clone();
            let click_key = o.key.clone();
            let correct_key = answer_keys.clone();
            view! {
              <button
                type="button"
                class=move || {
                  let base = "w-full border px-3 py-1.5 text-left text-sm transition-colors";
                  if answered() {
                    if correct_key.contains(&class_key) {
                      format!("{base} border-win bg-win/10")
                    } else if is_sel() {
                      format!("{base} border-destructive bg-destructive/10")
                    } else {
                      base.to_string()
                    }
                  } else if is_sel() {
                    format!("{base} border-primary bg-primary/10")
                  } else {
                    format!("{base} hover:bg-accent")
                  }
                }
                on:click=move |_| toggle(click_key.clone())
              >
                <span class="font-mono">{key_label}</span> "　" {text}
              </button>
            }
          })
          .collect_view()}
      </div>
      {move || {
        answered().then(|| {
          if correct() {
            view! { <div class="mt-1.5 text-xs text-win-text">{move || t("common.correct-verdict")}</div> }.into_any()
          } else {
            view! {
              <div class="mt-1.5 text-xs text-hp-text">
                {move || t("exam.correct-answer-prefix")} <span class="font-mono">{answer_keys.join("、")}</span>
              </div>
            }
            .into_any()
          }
        })
      }}
      // 作答后才出现，避免提前泄题。
      {move || {
        has_answer.get().then(|| {
          view! {
            <div class="mt-2">
              <ExplanationCard explanation=explain.clone() />
            </div>
          }
        })
      }}
    </div>
  }
}
