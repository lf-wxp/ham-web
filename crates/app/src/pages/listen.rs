//! 听题模式：自动朗读题干、选项，停顿思考后读出答案（可选解析），适合通勤时免手刷题。

use std::collections::BTreeMap;
use std::sync::Arc;

use ham_web_core::exam::shuffled;
use ham_web_core::listen::{Phase, RATE_CHOICES, Step, THINK_CHOICES, script};
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::SpeechSynthesisUtterance;

use super::print_sheet::{Source as Collection, load_items};
use crate::components::common::PageContainer;
use crate::data;
use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Checkbox, ControlSize, NativeSelect, SelectOption, Size, Variant};
use crate::util::{random, set_title, sleep, storage, window};

const KEY: &str = "listen-settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Source {
  A,
  B,
  C,
  Mistakes,
  Bookmarks,
}

impl Source {
  const ALL: [Self; 5] = [Self::A, Self::B, Self::C, Self::Mistakes, Self::Bookmarks];

  const fn label(self) -> &'static str {
    match self {
      Self::A => "A 类",
      Self::B => "B 类",
      Self::C => "C 类",
      Self::Mistakes => "错题集",
      Self::Bookmarks => "收藏集",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
  source: Source,
  rate: f32,
  think_secs: u32,
  explain: bool,
  shuffle: bool,
  /// 各来源上次听到的位置（顺序模式）。
  positions: BTreeMap<Source, usize>,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      source: Source::A,
      rate: 1.0,
      think_secs: 5,
      explain: false,
      shuffle: false,
      positions: BTreeMap::new(),
    }
  }
}

async fn load_source(source: Source) -> Vec<QuestionItem> {
  let bank = match source {
    Source::A => Some(Bank::A),
    Source::B => Some(Bank::B),
    Source::C => Some(Bank::C),
    Source::Mistakes | Source::Bookmarks => None,
  };
  if let Some(b) = bank {
    return data::load_bank(None, b, false)
      .await
      .map(|qs| qs.as_ref().clone())
      .unwrap_or_default();
  }
  let collection = if source == Source::Mistakes {
    Collection::Mistakes
  } else {
    Collection::Bookmarks
  };
  load_items(collection, None)
    .await
    .into_iter()
    .map(|it| it.q)
    .collect()
}

fn synth() -> Option<web_sys::SpeechSynthesis> {
  window().speech_synthesis().ok()
}

/// 朗读一段中文并等待结束；部分浏览器偶尔不触发 `end`，按字数设超时兜底。
async fn say(text: &str, rate: f32) {
  let (Some(synth), Ok(u)) = (synth(), SpeechSynthesisUtterance::new_with_text(text)) else {
    return;
  };
  u.set_lang(crate::speech::content_lang());
  u.set_rate(rate);
  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  let timeout = (text.chars().count() as f32 * 450.0 / rate) as i32 + 4000;
  let p = js_sys::Promise::new(&mut |resolve, _| {
    let done = Closure::once_into_js({
      let resolve = resolve.clone();
      move || {
        let _ = resolve.call0(&JsValue::NULL);
      }
    });
    u.set_onend(Some(done.unchecked_ref()));
    u.set_onerror(Some(done.unchecked_ref()));
    let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, timeout);
  });
  synth.speak(&u);
  let _ = JsFuture::from(p).await;
}

fn phase_label(phase: Option<Phase>) -> &'static str {
  match phase {
    None => "已暂停",
    Some(Phase::Question) => "朗读题目",
    Some(Phase::Think) => "思考中…",
    Some(Phase::Answer) => "公布答案",
    Some(Phase::Explain) => "朗读解析",
  }
}

#[component]
pub fn ListenPage() -> impl IntoView {
  set_title("shell.listening");
  let settings = RwSignal::new(storage::get_json::<Settings>(KEY).unwrap_or_default());
  let questions = RwSignal::new(Arc::new(Vec::<QuestionItem>::new()));
  let loading = RwSignal::new(true);
  let index = RwSignal::new(0usize);
  let phase = RwSignal::new(None::<Phase>);
  let generation = StoredValue::new(0u32);
  let supported = synth().is_some();

  let save = move || settings.with_untracked(|s| storage::set_json(KEY, s));

  let rate_options: Vec<SelectOption> = RATE_CHOICES
    .iter()
    .copied()
    .map(|r| SelectOption::new(r.to_string(), format!("{r}×")))
    .collect();
  let think_options: Vec<SelectOption> = THINK_CHOICES
    .iter()
    .copied()
    .map(|s| SelectOption::new(s.to_string(), tf("common.s", &[&s.to_string()])))
    .collect();

  let stop = move || {
    generation.update_value(|g| *g = g.wrapping_add(1));
    if let Some(s) = synth() {
      s.cancel();
    }
    phase.set(None);
  };

  // 只在来源或顺序变化时重新加载题目（记录播放位置也会改 settings）
  let selection = Memo::new(move |_| settings.with(|s| (s.source, s.shuffle)));
  Effect::new(move |_| {
    let (source, shuffle) = selection.get();
    stop();
    loading.set(true);
    spawn_local(async move {
      let mut qs = load_source(source).await;
      if shuffle {
        let mut rng = random;
        qs = shuffled(&qs, &mut rng);
      }
      let start = if shuffle {
        0
      } else {
        settings.with_untracked(|s| s.positions.get(&source).copied().unwrap_or(0))
      };
      index.set(start.min(qs.len().saturating_sub(1)));
      questions.set(Arc::new(qs));
      loading.set(false);
    });
  });

  let remember = move |i: usize| {
    settings.update(|s| {
      if !s.shuffle {
        s.positions.insert(s.source, i);
      }
    });
    save();
  };

  let play = move || {
    generation.update_value(|g| *g = g.wrapping_add(1));
    let my = generation.get_value();
    if let Some(s) = synth() {
      s.cancel();
    }
    spawn_local(async move {
      loop {
        let alive = || generation.try_get_value() == Some(my);
        let i = index.get_untracked();
        let Some(q) = questions.with_untracked(|qs| qs.get(i).cloned()) else {
          break;
        };
        remember(i);
        let (rate, think, explain) = settings.with_untracked(|s| (s.rate, s.think_secs, s.explain));
        for step in script(&q, i + 1, think, explain) {
          if !alive() {
            return;
          }
          phase.set(Some(step.phase()));
          match step {
            Step::Say(_, text) => say(&text, rate).await,
            Step::Pause(_, ms) => sleep(ms).await,
          }
        }
        if !alive() {
          return;
        }
        let total = questions.with_untracked(|qs| qs.len());
        if i + 1 >= total {
          break;
        }
        index.set(i + 1);
      }
      if generation.try_get_value() == Some(my) {
        phase.set(None);
      }
    });
  };

  let go = move |delta: isize| {
    let total = questions.with_untracked(|qs| qs.len());
    if total == 0 {
      return;
    }
    let playing = phase.get_untracked().is_some();
    let next = index
      .get_untracked()
      .saturating_add_signed(delta)
      .min(total - 1);
    index.set(next);
    remember(next);
    if playing {
      play();
    }
  };

  let toggle = move || {
    if phase.get_untracked().is_some() {
      stop();
    } else {
      play();
    }
  };

  on_cleanup(move || {
    if let Some(s) = synth() {
      s.cancel();
    }
  });

  let seg = |active: bool| {
    if active {
      "rounded-md bg-background px-2.5 py-1 text-xs font-medium shadow-sm"
    } else {
      "rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground"
    }
  };

  let card = move || {
    if loading.get() {
      return view! { <div class="p-6 text-sm text-muted-foreground" aria-live="polite">{move || t("exam.loading-questions-2")}</div> }.into_any();
    }
    let i = index.get();
    let Some(q) = questions.with(|qs| qs.get(i).cloned()) else {
      return view! { <div class="p-6 text-sm text-muted-foreground">{move || t("exam.no-questions-here-yet")}</div> }
        .into_any();
    };
    let total = questions.with(|qs| qs.len());
    let revealed = move || matches!(phase.get(), Some(Phase::Answer | Phase::Explain));
    let answer = q.answer_keys.join("、");
    let answer_keys = q.answer_keys.clone();
    view! {
      <div class="p-5">
        <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
          <span class="tabular-nums">{tf("common.question", &[&(i + 1).to_string(), &total.to_string(), &(t(q.kind.label())).to_string()])}</span>
          <span
            class=move || if phase.get().is_some() { "rounded-full bg-primary/10 px-2 py-0.5 font-medium text-foreground" } else { "px-2 py-0.5" }
            aria-live="polite"
          >
            {move || t(phase_label(phase.get()))}
          </span>
        </div>
        <p class="whitespace-pre-line text-base font-medium leading-relaxed">{q.question.clone()}</p>
        <ul class="mt-3 space-y-1.5 text-sm">
          {q.options.iter().map(|o| {
            let correct = answer_keys.contains(&o.key);
            let key = o.key.clone();
            view! {
              <li class=move || if revealed() && correct { "rounded-md bg-emerald-500/10 px-2 py-1 font-medium text-emerald-800 dark:text-emerald-300" } else { "px-2 py-1" }>
                <span class="font-mono">{key.clone()}</span> "　" {o.text.clone()}
              </li>
            }
          }).collect_view()}
        </ul>
        <div class="mt-3 min-h-5 text-sm">
          {move || revealed().then(|| view! { <span>{move || t("exam.correct-answer-2")} <span class="font-mono font-semibold">{answer.clone()}</span></span> })}
        </div>
      </div>
    }
    .into_any()
  };

  view! {
    <PageContainer class="space-y-4 py-6 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div>
        <h1 class="text-base font-semibold leading-tight">{move || t("shell.listening")}</h1>
        <p class="text-xs text-muted-foreground">{move || t("exam.reads-the-question-and")}</p>
      </div>

      {(!supported).then(|| view! {
        <div role="alert" class="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-800 dark:text-amber-300">
          {move || t("exam.this-browser-does-not")}
        </div>
      })}

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <div class="inline-flex rounded-lg bg-muted p-0.5" role="group" aria-label=move || t("exam.question-source")>
          {Source::ALL.into_iter().map(|s| view! {
            <button
              type="button"
              class=move || seg(settings.with(|st| st.source == s))
              aria-pressed=move || settings.with(|st| st.source == s).to_string()
              on:click=move |_| { settings.update(|st| st.source = s); save(); }
            >
              {move || t(s.label())}
            </button>
          }).collect_view()}
        </div>
        <label class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <Checkbox
            checked=Signal::derive(move || settings.with(|s| s.shuffle))
            on_change=Callback::new(move |on: bool| {
              settings.update(|s| s.shuffle = on);
              save();
            })
          />
          {move || t("exam.shuffle")}
        </label>
        <label class="inline-flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground">
          <Checkbox
            checked=Signal::derive(move || settings.with(|s| s.explain))
            on_change=Callback::new(move |on: bool| {
              settings.update_untracked(|s| s.explain = on);
              save();
            })
          />
          {move || t("learning.read-explanation-aloud")}
        </label>
      </div>

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-muted-foreground">
        <label class="inline-flex items-center gap-1.5">
          {move || t("exam.speed")}
          <NativeSelect
            value=Signal::derive(move || {
              settings.with(|s| {
                RATE_CHOICES
                  .iter()
                  .copied()
                  .find(|r| (r - s.rate).abs() < 0.01)
                  .map_or_else(|| s.rate.to_string(), |r| r.to_string())
              })
            })
            on_change=Callback::new(move |v: String| {
              let rate: f32 = v.parse().unwrap_or(1.0);
              settings.update_untracked(|s| s.rate = rate);
              save();
            })
            options=rate_options
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("exam.speed"))
            class="w-auto"
          />
        </label>
        <label class="inline-flex items-center gap-1.5">
          {move || t("exam.think-time")}
          <NativeSelect
            value=Signal::derive(move || settings.with(|s| s.think_secs.to_string()))
            on_change=Callback::new(move |v: String| {
              let secs: u32 = v.parse().unwrap_or(5);
              settings.update_untracked(|s| s.think_secs = secs);
              save();
            })
            options=think_options
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("exam.think-time"))
            class="w-auto"
          />
        </label>
      </div>

      <section class="rounded-xl border bg-card" aria-label=move || t("exam.current-question")>{card}</section>

      <div class="flex items-center justify-center gap-3">
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| go(-1))
        >{move || t("exam.previous")}</Button>
        <Button
          variant=Variant::Default
          size=Size::Default
          class="min-w-28"
          disabled=Signal::derive(move || !supported || loading.get() || questions.with(|q| q.is_empty()))
          on_click=Callback::new(move |_| toggle())
        >
          <Icon kind=IconKind::Headphones class="h-4 w-4" />
          {move || if phase.get().is_some() { t("exam.pause") } else { t("exam.start-listening") }}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| go(1))
        >{move || t("exam.next")}</Button>
      </div>
      <p class="text-center text-xs text-muted-foreground">{move || t("exam.some-phones-pause-speech")}</p>
    </PageContainer>
  }
}
