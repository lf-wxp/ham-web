use std::time::Duration;

use ham_web_core::koch::farnsworth;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::{morse_char_times, play_morse_timed_with};
use crate::morse_settings::use_morse_settings;
use crate::ui::{Button, ButtonKind, Input, Slider, Variant};
use crate::util::random;

use super::stats_panel::MorseStatsPanel;
use super::{
  Feedback, MODES, Mode, MorseStats, Question, Scope, Target, TrainerSettings, load_stats,
  load_trainer_settings, morse_display, pill_class, random_choice, random_question, roll_today,
  save_stats, save_trainer_settings,
};
use crate::i18n::t;

/// 目标元素是否可输入/可交互（用于快捷键跳过）。
fn is_interactive(e: &KeyboardEvent) -> bool {
  let Some(el) = e
    .target()
    .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
  else {
    return false;
  };
  let tag = el.tag_name().to_lowercase();
  matches!(
    tag.as_str(),
    "input" | "textarea" | "select" | "button" | "a"
  ) || el.is_content_editable()
}

#[component]
pub(super) fn MorseTrainer() -> impl IntoView {
  let settings = use_morse_settings();
  let initial = load_trainer_settings();
  let mode = RwSignal::new(initial.mode);
  let scope = RwSignal::new(initial.scope);
  let target = RwSignal::new(initial.target);
  let wpm = RwSignal::new(initial.wpm);
  let eff_wpm = RwSignal::new(initial.eff_wpm);
  let adaptive = RwSignal::new(initial.adaptive);
  let current = RwSignal::new(Question {
    text: "A".to_owned(),
    code: ".-".to_owned(),
  });
  let options = RwSignal::new(Vec::<String>::new());
  let answer = RwSignal::new(String::new());
  let feedback = RwSignal::new(None::<Feedback>);
  let stats = RwSignal::new(load_stats());
  let input_ref = NodeRef::<html::Input>::new();
  let playing = RwSignal::new(false);
  let read_highlight = RwSignal::new(None::<usize>);
  // 播放 / 换题的代数计数：用于失效旧的定时器，避免连续操作下状态错乱。
  let play_gen = RwSignal::new(0u32);
  let advance_gen = RwSignal::new(0u32);

  let persist = move || {
    save_trainer_settings(&TrainerSettings {
      mode: mode.get(),
      scope: scope.get(),
      target: target.get(),
      wpm: wpm.get(),
      eff_wpm: eff_wpm.get(),
      adaptive: adaptive.get(),
    });
  };

  // 生成题目与选项（选择模式带干扰项）。
  let make_question = move |scope: Scope, target: Target, m: Mode| -> (Question, Vec<String>) {
    let s = stats.get_untracked();
    if m == Mode::Choice {
      let (q, opts, _) = random_choice(scope, target, &s.mistakes, true, &mut random);
      (q, opts)
    } else {
      (
        random_question(scope, target, &s.mistakes, true, &mut random),
        Vec::new(),
      )
    }
  };

  // 播放当前题目并驱动「播放中」状态（脉冲 + 图标切换 + 看码逐字符高亮）。
  let play_current = move || {
    let question = current.get();
    let timing = farnsworth(wpm.get(), eff_wpm.get());
    let seq = play_gen.get_untracked().wrapping_add(1);
    play_gen.set(seq);
    let dur = play_morse_timed_with(
      &question.code,
      timing,
      settings.tone_hz(),
      settings.volume(),
    );
    // 看码模式逐字符高亮（听译/选择模式下不显示代码，无副作用）
    let chars = morse_char_times(&question.code, timing);
    for (i, &(start, _)) in chars.iter().enumerate() {
      set_timeout(
        move || {
          if play_gen.get_untracked() == seq {
            read_highlight.set(Some(i));
          }
        },
        Duration::from_secs_f64(start),
      );
    }
    let total = chars.last().map_or(0.12, |(_, e)| *e);
    set_timeout(
      move || {
        if play_gen.get_untracked() == seq {
          read_highlight.set(None);
        }
      },
      Duration::from_secs_f64(total.max(0.12)),
    );
    playing.set(true);
    set_timeout(
      move || {
        if play_gen.get_untracked() == seq {
          playing.set(false);
        }
      },
      Duration::from_secs_f64(dur.max(0.12)),
    );
  };

  let (q, opts) = make_question(initial.scope, initial.target, initial.mode);
  current.set(q);
  options.set(opts);

  let load_next = move || {
    // 失效任何待执行的自动换题定时器
    advance_gen.update(|g| *g = g.wrapping_add(1));
    let (q, opts) = make_question(scope.get(), target.get(), mode.get());
    current.set(q);
    options.set(opts);
    answer.set(String::new());
    feedback.set(None);
    read_highlight.set(None);
    if let Some(el) = input_ref.get() {
      let _ = el.focus();
    }
  };

  let submit_guess = move |raw: String| {
    let question = current.get();
    let guess: String = raw
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
      let w = wpm.get_untracked();
      stats.update(|s| {
        roll_today(s);
        s.correct += 1;
        s.today_correct += 1;
        s.streak += 1;
        s.best_streak = s.best_streak.max(s.streak);
        s.top_wpm = s.top_wpm.max(w);
      });
      if adaptive.get_untracked() {
        wpm.update(|v| *v = (*v + 1.0).min(40.0));
        persist();
      }
      save_stats(&stats.get_untracked());
      feedback.set(Some(Feedback {
        ok: true,
        question: question.clone(),
      }));
      let scope = scope.get();
      let target = target.get();
      let m = mode.get();
      let seq = advance_gen.get_untracked();
      set_timeout(
        move || {
          if advance_gen.get_untracked() != seq {
            return;
          }
          let (q, opts) = make_question(scope, target, m);
          current.set(q);
          options.set(opts);
          answer.set(String::new());
          feedback.set(None);
        },
        Duration::from_millis(700),
      );
    } else {
      stats.update(|s| {
        roll_today(s);
        s.wrong += 1;
        s.today_wrong += 1;
        s.streak = 0;
        *s.mistakes.entry(question.text.clone()).or_default() += 1;
      });
      if adaptive.get_untracked() {
        wpm.update(|v| *v = (*v - 2.0).max(5.0));
        persist();
      }
      save_stats(&stats.get_untracked());
      feedback.set(Some(Feedback {
        ok: false,
        question: question.clone(),
      }));
      // 清空输入，避免重复回车重复计错
      answer.set(String::new());
      if let Some(el) = input_ref.get() {
        let _ = el.focus();
      }
    }
  };

  let submit = move || submit_guess(answer.get());

  let hint = move || {
    let question = current.get();
    feedback.set(Some(Feedback {
      ok: false,
      question: question.clone(),
    }));
  };

  let reset = move || {
    let empty = MorseStats::default();
    stats.set(empty.clone());
    save_stats(&empty);
    load_next();
  };
  let on_reset = Callback::new(move |()| reset());

  // 快捷键：空格重播、→ 下一题。
  window_event_listener(leptos::ev::keydown, move |e| {
    if is_interactive(&e) {
      return;
    }
    match e.key().as_str() {
      " " => {
        e.prevent_default();
        play_current();
      }
      "ArrowRight" => {
        e.prevent_default();
        load_next();
      }
      k if k.len() == 1
        && matches!(k.as_bytes()[0], b'1'..=b'4')
        && mode.get_untracked() == Mode::Choice =>
      {
        let n = usize::from(k.as_bytes()[0] - b'0');
        if let Some(opt) = options.get_untracked().get(n.saturating_sub(1)) {
          e.prevent_default();
          submit_guess(opt.clone());
        }
      }
      _ => {}
    }
  });

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("morse.decoding-practice")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.space-to-replay-next")}</span>
      </h2>

      <div class="space-y-4 p-4">
        // 控制区
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
          <div class="flex shrink-0 items-center gap-1.5">
            <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("log.mode")}</span>
            {MODES
              .iter()
              .map(|&m| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      mode.set(m);
                      persist();
                      load_next();
                    }
                    aria-pressed=move || (mode.get() == m).to_string()
                    class=move || pill_class(mode.get() == m)
                  >
                    {m.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="flex shrink-0 items-center gap-1.5">
            <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("radio.type")}</span>
            {Target::ALL
              .iter()
              .map(|&t| {
                view! {
                  <button
                    type="button"
                    prop:disabled=move || scope.get() == Scope::Weak
                    aria-disabled=move || (scope.get() == Scope::Weak).to_string()
                    on:click=move |_| {
                      target.set(t);
                      persist();
                      load_next();
                    }
                    aria-pressed=move || (target.get() == t).to_string()
                    class=move || {
                      let base = pill_class(target.get() == t);
                      if scope.get() == Scope::Weak {
                        format!("{base} opacity-40")
                      } else {
                        base.to_owned()
                      }
                    }
                  >
                    {t.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="flex shrink-0 items-center gap-1.5">
            <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("morse.scope")}</span>
            {Scope::ALL
              .iter()
              .map(|&s| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      scope.set(s);
                      if s == Scope::Weak {
                        target.set(Target::Single);
                      }
                      persist();
                      load_next();
                    }
                    aria-pressed=move || (scope.get() == s).to_string()
                    class=move || pill_class(scope.get() == s)
                  >
                    {s.label()}
                  </button>
                }
              })
              .collect_view()}
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("radio.speed")}</span>
            <Slider
              value=wpm
              on_change=Callback::new(move |v: f64| {
                wpm.set(v);
                persist();
              })
              min=5.0
              max=40.0
              step=1.0
              class="w-32"
              aria_label=Signal::derive(move || t("morse.sending-speed-wpm"))
              aria_valuetext=Signal::derive(move || format!("{} WPM", wpm.get().round()))
            />
            <span class="text-xs tabular-nums text-muted-foreground">{move || wpm.get().round()} " WPM"</span>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("morse.effective-speed")}</span>
            <Slider
              value=eff_wpm
              on_change=Callback::new(move |v: f64| {
                eff_wpm.set(v);
                persist();
              })
              min=5.0
              max=40.0
              step=1.0
              class="w-32"
              aria_label=Signal::derive(move || t("morse.effective-speed-wpm-farnsworth"))
              aria_valuetext=Signal::derive(move || format!("{} WPM", eff_wpm.get().round()))
            />
            <span class="text-xs tabular-nums text-muted-foreground">{move || eff_wpm.get().round()} " WPM"</span>
          </div>

          <button
            type="button"
            on:click=move |_| {
              adaptive.update(|v| *v = !*v);
              persist();
            }
            aria-pressed=move || adaptive.get().to_string()
            class=move || pill_class(adaptive.get())
            title=move || t("morse.speed-up-on-correct")
          >
            {move || t("morse.adaptive-speed")}
          </button>
        </div>

        // 题目区
        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-8">
          {move || {
            let question = current.get();
            if mode.get() == Mode::Read {
              let tokens: Vec<String> = question
                .code
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned())
                .collect();
              let compact = target.get() == Target::Sentence;
              view! {
                <div
                  class=if compact {
                    "flex max-w-2xl flex-wrap items-center justify-center gap-x-2 gap-y-1.5 font-mono text-lg font-semibold leading-relaxed tracking-[0.15em]"
                  } else {
                    "flex flex-wrap items-center justify-center gap-x-1.5 font-mono text-3xl font-semibold tracking-[0.3em]"
                  }
                >
                  {tokens
                    .into_iter()
                    .enumerate()
                    .map(|(i, tok)| {
                      view! {
                        <span class=if read_highlight.get() == Some(i) { "text-emerald-500 dark:text-emerald-400" } else { "text-primary" }>
                          {morse_display(&tok)}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
                <button
                  type="button"
                  on:click=move |_| play_current()
                  class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border bg-card px-3 py-1 text-xs transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                >
                  <Icon kind=IconKind::Volume2 class="h-3.5 w-3.5" />
                  {move || t("exam.preview")}
                </button>
              }
              .into_any()
            } else {
              let hint = if mode.get() == Mode::Choice {
                t("morse.tap-play-then-choose")
              } else {
                t("morse.tap-play-and-type")
              };
              view! {
                <button
                  type="button"
                  aria-label=move || t("morse.play")
                  on:click=move |_| play_current()
                  class="relative flex size-20 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg shadow-primary/25 transition-all duration-200 ease-out hover:scale-105 hover:shadow-xl hover:shadow-primary/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background active:scale-95"
                >
                  {move || {
                    playing.get().then(|| {
                      view! {
                        <span class="absolute inset-0 rounded-full bg-primary/40 animate-ping [animation-duration:1.4s]"></span>
                      }
                    })
                  }}
                  <span class="relative">
                    {move || {
                      if playing.get() {
                        view! { <Icon kind=IconKind::AudioLines class="h-8 w-8" /> }.into_any()
                      } else {
                        view! { <Icon kind=IconKind::Play class="h-8 w-8" /> }.into_any()
                      }
                    }}
                  </span>
                </button>
                <div class="text-xs text-muted-foreground">{hint}</div>
              }
              .into_any()
            }
          }}
        </div>

        // 输入 / 选项与反馈
        <div class="flex flex-col items-center gap-3">
          {move || {
            if mode.get() == Mode::Choice {
              view! {
                <div class="grid w-full max-w-md grid-cols-1 gap-2 sm:grid-cols-2">
                  {options
                    .get()
                    .iter()
                    .map(|opt| {
                      let text = opt.clone();
                      view! {
                        <button
                          type="button"
                          on:click=move |_| submit_guess(text.clone())
                          class="inline-flex min-h-10 items-center justify-center rounded-lg border bg-card px-3 py-2 font-mono text-sm font-semibold uppercase tracking-wide transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                        >
                          {text.clone()}
                        </button>
                      }
                    })
                    .collect_view()}
                </div>
                <div class="flex flex-wrap items-center justify-center gap-2">
                  <Button
                    variant=Variant::Outline
                    class="rounded-lg h-10"
                    on_click=Callback::new(move |_| load_next())
                  >{move || t("exam.skip")}</Button>
                  <Button
                    variant=Variant::Outline
                    class="rounded-lg h-10"
                    on_click=Callback::new(move |_| load_next())
                  >{move || t("exam.next")}</Button>
                </div>
              }
              .into_any()
            } else {
              // 宽度随目标文案类型变化，而 `Input` 的 `class` 是静态串 —— 在外层闭包里先算出来
              // （闭包会随 `target` 变化重建，取到新宽度）。
              let width = if target.get() == Target::Sentence { "w-64 sm:w-80" } else { "w-32" };
              let answer_class = format!("h-10 {width} px-3 text-center text-lg font-semibold uppercase");
              view! {
                <form
                  class="flex flex-wrap items-center justify-center gap-2"
                  on:submit=move |e| {
                    e.prevent_default();
                    submit();
                  }
                >
                  <Input
                    node_ref=input_ref
                    value=answer
                    on_change=Callback::new(move |v: String| answer.set(v))
                    placeholder=Signal::derive(move || target.get().placeholder().to_owned())
                    maxlength=Signal::derive(move || target.get().max_len().to_string())
                    aria_label=Signal::derive(move || t("morse.copy-input"))
                    autocomplete="off"
                    class=answer_class
                  />
                  <Button
                    kind=ButtonKind::Submit
                    class="rounded-lg h-10"
                    disabled=Signal::derive(move || answer.get().trim().is_empty())
                  >
                    {move || t("learning.submit")}
                  </Button>
                  <Button
                    variant=Variant::Outline
                    class="rounded-lg h-10"
                    on_click=Callback::new(move |_| hint())
                  >{move || t("exam.notice")}</Button>
                  <Button
                    variant=Variant::Outline
                    class="rounded-lg h-10"
                    on_click=Callback::new(move |_| load_next())
                  >{move || t("exam.skip")}</Button>
                  <Button
                    variant=Variant::Outline
                    class="rounded-lg h-10"
                    on_click=Callback::new(move |_| load_next())
                  >{move || t("exam.next")}</Button>
                </form>
              }
              .into_any()
            }
          }}

          <div class="min-h-5 text-sm" aria-live="polite">
            {move || {
              feedback.get().map(|fb| {
                if fb.ok {
                  view! {
                    <span class="inline-flex items-center gap-1.5 font-medium text-emerald-600 dark:text-emerald-400 animate-in fade-in zoom-in-95 duration-200">
                      <Icon kind=IconKind::CheckCircle2 class="h-4 w-4" />
                      {move || t("learning.correct")}
                    </span>
                  }
                  .into_any()
                } else {
                  view! {
                    <span class="inline-flex items-center gap-1.5 font-medium text-red-600 dark:text-red-400 animate-in fade-in zoom-in-95 duration-200">
                      <Icon kind=IconKind::XCircle class="h-4 w-4" />
                      {move || t("morse.the-answer-is")} <span class="font-mono font-semibold">{fb.question.text}</span> "　"
                      <span class="font-mono">{morse_display(&fb.question.code)}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>

          <MorseStatsPanel stats=stats on_reset=on_reset />
        </div>
      </div>
    </section>
  }
}
