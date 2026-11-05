//! 单道自测题：独立管理自己的作答与判分状态。

use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::components::common::ExplanationCard;
use crate::i18n::t;

/// 一道自测题（独立组件，管理自己的作答状态）。
#[component]
pub(super) fn QuizQuestion(
  q: QuestionItem,
  /// 题号（1 起）：自测三题连排时要能看出「这是第几题」。
  index: usize,
  /// 判分结果上报给父组件（用于顶部「已完成 / 正确」计数）。
  #[prop(optional, into)]
  on_verdict: Option<Callback<bool>>,
) -> impl IntoView {
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
  let q_check = q.clone();
  let correct = move || q.is_answer_correct(&selected.get());

  // 判分只上报一次：多选反复勾选时不要让计数抖动。
  //
  // 「什么时候算答完」：单选点一下就是答案；多选要选满标准答案的个数才判 —— 点第一个正确项
  // 就上报会被记成错（此时只选了一半），而下方的对错提示随后又会变成「正确」，计分与提示不一致。
  // 两套皮肤共用这一份：像素风以前根本不上报，父组件的「已完成 / 正确」计数永远是 0。
  let reported = RwSignal::new(false);
  let expected = answer_keys.len();
  let report = Callback::new(move |()| {
    if reported.get_untracked() {
      return;
    }
    let chosen = selected.get_untracked();
    let ready = if is_multiple {
      chosen.len() >= expected
    } else {
      !chosen.is_empty()
    };
    if !ready {
      return;
    }
    reported.set(true);
    if let Some(cb) = on_verdict {
      cb.run(q_check.is_answer_correct(&chosen));
    }
  });
  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-quiz" data-answered=move || answered().to_string()>
        <div class="md-quiz-head">
          <span class="md-quiz-num" aria-hidden="true">{index}</span>
          <p class="md-quiz-q">{question}</p>
          {is_multiple
            .then(|| {
              view! { <span class="md-quiz-tag">{move || t("exam.multiple")}</span> }
            })}
        </div>
        <div class="md-quiz-options">
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
                  class="md-quiz-opt"
                  data-state=move || {
                    if answered() {
                      if correct_key.contains(&class_key) {
                        "right"
                      } else if is_sel() {
                        "wrong"
                      } else {
                        "idle"
                      }
                    } else if is_sel() {
                      "sel"
                    } else {
                      "idle"
                    }
                  }
                  on:click=move |_| {
                    toggle(click_key.clone());
                    report.run(());
                  }
                >
                  <span class="md-quiz-key">{key_label}</span>
                  <span class="md-quiz-text">{text}</span>
                  <span class="md-quiz-mark" aria-hidden="true"></span>
                </button>
              }
            })
            .collect_view()}
        </div>
        {move || {
          answered()
            .then(|| {
              if correct() {
                view! {
                  <p class="md-quiz-verdict" data-ok="true">
                    {move || t("common.correct-verdict")}
                  </p>
                }
                  .into_any()
              } else {
                view! {
                  <p class="md-quiz-verdict" data-ok="false">
                    {move || t("exam.correct-answer-prefix")} " "
                    <span class="md-mono">{answer_keys.join("、")}</span>
                  </p>
                }
                  .into_any()
              }
            })
        }}
        {move || {
          has_answer
            .get()
            .then(|| {
              view! {
                <div class="mt-2">
                  <ExplanationCard explanation=explain.clone() />
                </div>
              }
            })
        }}
      </div>
    }
    .into_any();
  }
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
                  let base = "w-full border px-3 py-1.5 text-left text-sm transition-colors pointer-coarse:min-h-11";
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
                on:click=move |_| {
                  toggle(click_key.clone());
                  report.run(());
                }
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
  .into_any()
}
