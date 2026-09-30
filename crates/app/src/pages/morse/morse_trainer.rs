use std::time::Duration;

use leptos::html;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_morse;

use super::{
  Feedback, MODES, Mode, MorseStats, Question, Scope, Target, load_stats, morse_display,
  pill_class, random_question, save_stats,
};

#[component]
pub(super) fn MorseTrainer() -> impl IntoView {
  let mode = RwSignal::new(Mode::Listen);
  let scope = RwSignal::new(Scope::Letters);
  let target = RwSignal::new(Target::Single);
  let weak_first = RwSignal::new(false);
  let wpm = RwSignal::new(20.0);
  let current = RwSignal::new(Question {
    text: "A".to_owned(),
    code: ".-".to_owned(),
  });
  let answer = RwSignal::new(String::new());
  let feedback = RwSignal::new(None::<Feedback>);
  let stats = RwSignal::new(load_stats());
  let input_ref = NodeRef::<html::Input>::new();

  current.set(random_question(
    Scope::Letters,
    Target::Single,
    &stats.get_untracked().mistakes,
    weak_first.get_untracked(),
  ));

  let load_next = move || {
    let s = stats.get_untracked();
    current.set(random_question(
      scope.get(),
      target.get(),
      &s.mistakes,
      weak_first.get(),
    ));
    answer.set(String::new());
    feedback.set(None);
    if let Some(el) = input_ref.get() {
      let _ = el.focus();
    }
  };

  let submit = move || {
    let question = current.get();
    let guess: String = answer
      .get()
      .chars()
      .filter(|c| !c.is_whitespace())
      .map(|c| c.to_ascii_uppercase())
      .collect();
    if guess.is_empty() {
      return;
    }
    let expected: String = question
      .text
      .chars()
      .filter(|c| !c.is_whitespace())
      .collect();
    if guess == expected {
      stats.update(|s| s.correct += 1);
      save_stats(&stats.get_untracked());
      feedback.set(Some(Feedback {
        ok: true,
        question: question.clone(),
      }));
      let scope = scope.get();
      let target = target.get();
      set_timeout(
        move || {
          let s = stats.get_untracked();
          current.set(random_question(
            scope,
            target,
            &s.mistakes,
            weak_first.get_untracked(),
          ));
          answer.set(String::new());
          feedback.set(None);
        },
        Duration::from_millis(700),
      );
    } else {
      stats.update(|s| {
        s.wrong += 1;
        *s.mistakes.entry(question.text.clone()).or_default() += 1;
      });
      save_stats(&stats.get_untracked());
      feedback.set(Some(Feedback {
        ok: false,
        question: question.clone(),
      }));
    }
  };

  let hint = move || {
    let question = current.get();
    feedback.set(Some(Feedback {
      ok: false,
      question: question.clone(),
    }));
  };

  let skip = move || load_next();

  let reset = move || {
    let empty = MorseStats::default();
    stats.set(empty.clone());
    save_stats(&empty);
    load_next();
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"解码练习"</h2>

      <div class="space-y-4 p-4">
        // 控制区
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
          <div class="flex items-center gap-1.5">
            <span class="text-xs text-muted-foreground">"模式"</span>
            {MODES
              .iter()
              .map(|&m| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| mode.set(m)
                    class=move || pill_class(mode.get() == m)
                  >
                    {m.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="flex items-center gap-1.5">
            <span class="text-xs text-muted-foreground">"类型"</span>
            {Target::ALL
              .iter()
              .map(|&t| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      target.set(t);
                      load_next();
                    }
                    class=move || pill_class(target.get() == t)
                  >
                    {t.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="flex items-center gap-1.5">
            <span class="text-xs text-muted-foreground">"范围"</span>
            {Scope::ALL
              .iter()
              .map(|&s| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      scope.set(s);
                      load_next();
                    }
                    class=move || pill_class(scope.get() == s)
                  >
                    {s.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <button
            type="button"
            on:click=move |_| weak_first.update(|v| *v = !*v)
            class=move || pill_class(weak_first.get())
            title="开启后优先出答错过的字符"
          >
            "易错优先"
          </button>

          <div class="flex items-center gap-2">
            <span class="text-xs text-muted-foreground">"速度"</span>
            <input
              type="range"
              min="5"
              max="40"
              step="1"
              prop:value=move || wpm.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  wpm.set(v);
                }
              }
              class="h-1.5 w-32 accent-primary"
            />
            <span class="text-xs tabular-nums text-muted-foreground">{move || wpm.get().round()} " WPM"</span>
          </div>
        </div>

        // 题目区
        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-8">
          {move || {
            let question = current.get();
            if mode.get() == Mode::Listen {
              view! {
                <button
                  type="button"
                  aria-label="播放"
                  on:click=move |_| play_morse(&question.code, wpm.get())
                  class="flex size-20 items-center justify-center rounded-full bg-primary text-primary-foreground shadow transition-transform hover:scale-105 active:scale-95"
                >
                  <Icon kind=IconKind::Play class="h-8 w-8" />
                </button>
                <div class="text-xs text-muted-foreground">"点击播放，听出内容后输入"</div>
              }
              .into_any()
            } else {
              view! {
                <div class="font-mono text-3xl font-semibold tracking-[0.3em] text-primary">
                  {morse_display(&question.code)}
                </div>
                <button
                  type="button"
                  on:click=move |_| play_morse(&question.code, wpm.get())
                  class="rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
                >
                  "试听"
                </button>
              }
              .into_any()
            }
          }}
        </div>

        // 输入与反馈
        <div class="flex flex-col items-center gap-3">
          <form
            class="flex flex-wrap items-center justify-center gap-2"
            on:submit=move |e| {
              e.prevent_default();
              submit();
            }
          >
            <input
              node_ref=input_ref
              prop:value=move || answer.get()
              on:input=move |e| answer.set(event_target_value(&e))
              placeholder=move || target.get().placeholder()
              maxlength=move || target.get().max_len()
              autocomplete="off"
              class="h-10 w-28 rounded-lg border bg-background text-center text-lg font-semibold uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            />
            <button
              type="submit"
              class="h-10 shrink-0 whitespace-nowrap rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
            >
              "提交"
            </button>
            <button
              type="button"
              on:click=move |_| hint()
              class="h-10 shrink-0 whitespace-nowrap rounded-lg border px-3 text-sm transition-colors hover:bg-accent"
            >
              "提示"
            </button>
            <button
              type="button"
              on:click=move |_| skip()
              class="h-10 shrink-0 whitespace-nowrap rounded-lg border px-3 text-sm transition-colors hover:bg-accent"
            >
              "跳过"
            </button>
            <button
              type="button"
              on:click=move |_| load_next()
              class="h-10 shrink-0 whitespace-nowrap rounded-lg border px-3 text-sm transition-colors hover:bg-accent"
            >
              "下一题"
            </button>
          </form>

          <div class="min-h-5 text-sm">
            {move || {
              feedback.get().map(|fb| {
                if fb.ok {
                  view! {
                    <span class="font-medium text-emerald-600 dark:text-emerald-400">"正确！"</span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="font-medium text-red-600 dark:text-red-400">
                      "答案是 " <span class="font-mono font-semibold">{fb.question.text}</span> "　"
                      <span class="font-mono">{morse_display(&fb.question.code)}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>

          <div class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
            <span>
              "正确 " <span class="font-semibold tabular-nums text-foreground">{move || stats.get().correct}</span>
            </span>
            <span>
              "错误 " <span class="font-semibold tabular-nums text-foreground">{move || stats.get().wrong}</span>
            </span>
            {move || {
              let weak: Vec<String> = stats
                .get()
                .mistakes
                .iter()
                .filter(|(_, n)| **n >= 2)
                .map(|(c, _)| c.clone())
                .collect();
              (!weak.is_empty()).then(|| {
                view! {
                  <span>
                    "易错：" <span class="font-mono font-semibold text-foreground">{weak.join(" ")}</span>
                  </span>
                }
              })
            }}
            <button
              type="button"
              on:click=move |_| reset()
              class="rounded-md px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
            >
              "重置"
            </button>
          </div>
        </div>
      </div>
    </section>
  }
}
