//! 单道自测题：独立管理自己的作答与判分状态。

use ham_web_core::QuestionItem;
use leptos::prelude::*;

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
  let correct = move || q.is_answer_correct(&selected.get());

  view! {
    <div class="rounded-lg border p-3">
      <div class="text-sm font-medium">{question}</div>
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
                  let base = "w-full rounded-lg border px-3 py-1.5 text-left text-sm transition-colors";
                  if answered() {
                    if correct_key.contains(&class_key) {
                      format!("{base} border-emerald-500 bg-emerald-50 dark:bg-emerald-950/40")
                    } else if is_sel() {
                      format!("{base} border-red-400 bg-red-50 dark:bg-red-950/40")
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
            view! { <div class="mt-1.5 text-xs text-emerald-600">{move || t("答对 ✓")}</div> }.into_any()
          } else {
            view! {
              <div class="mt-1.5 text-xs text-red-600">
                {move || t("正确答案：")} <span class="font-mono font-semibold">{answer_keys.join("、")}</span>
              </div>
            }
            .into_any()
          }
        })
      }}
    </div>
  }
}
