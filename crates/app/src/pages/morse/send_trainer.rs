use ham_web_core::morse::{DIGITS, LETTERS, MorseChar, code_of};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::icons::{Icon, IconKind};
use crate::morse_audio::{start_tone, stop_tone};
use crate::morse_settings::use_morse_settings;
use crate::util::storage;

use super::{morse_display, pill_class, random_index};
use crate::i18n::t;
use crate::ui::{Button, Variant};

/// 发报目标类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SendTarget {
  Char,
  Word,
  CallSign,
}

impl SendTarget {
  const fn label(self) -> &'static str {
    match self {
      Self::Char => "单字符",
      Self::Word => "单词",
      Self::CallSign => "呼号",
    }
  }

  const ALL: [Self; 3] = [Self::Char, Self::Word, Self::CallSign];
}

/// 发报统计（持久化）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SendStats {
  correct: usize,
  wrong: usize,
  best_streak: usize,
}

const SEND_KEY: &str = "morse-send";

fn load_send() -> SendStats {
  storage::get_json(SEND_KEY).unwrap_or_default()
}

fn save_send(s: &SendStats) {
  storage::set_json(SEND_KEY, s);
}

/// 按键时值统计（用于估算 WPM 与节奏评分）。
#[derive(Debug, Clone, Copy, Default)]
struct KeyingStats {
  dot_sum: f64,
  dot_sum2: f64,
  dot_n: u32,
  dash_sum: f64,
  dash_n: u32,
}

impl KeyingStats {
  fn wpm(self) -> Option<f64> {
    (self.dot_n > 0).then(|| 1200.0 / (self.dot_sum / f64::from(self.dot_n)))
  }

  fn ratio(self) -> Option<f64> {
    (self.dot_n > 0 && self.dash_n > 0)
      .then(|| (self.dash_sum / f64::from(self.dash_n)) / (self.dot_sum / f64::from(self.dot_n)))
  }

  /// 节奏评分（0–100）：综合点划比例（应 ≈1:3）与点长一致性。
  fn rhythm_score(self) -> Option<f64> {
    let ratio = self.ratio()?;
    let mean = self.dot_sum / f64::from(self.dot_n);
    let var = (self.dot_sum2 / f64::from(self.dot_n) - mean * mean).max(0.0);
    let cv = var.sqrt() / mean;
    let ratio_err = (ratio - 3.0).abs();
    Some((100.0 - ratio_err * 30.0 - cv * 60.0).clamp(0.0, 100.0))
  }
}

/// 发报练习：按住按钮拍发（带侧音），短按为点、长按为划，松开后判定；估算 WPM 与节奏。
#[component]
pub(super) fn SendTrainer() -> impl IntoView {
  let settings = use_morse_settings();
  let send_target = RwSignal::new(SendTarget::Char);
  let target = RwSignal::new("A".to_owned());
  let marks = RwSignal::new(String::new());
  let press_time = RwSignal::new(None::<f64>);
  let feedback = RwSignal::new(None::<bool>);
  let stats = RwSignal::new(load_send());
  let streak = RwSignal::new(0usize);
  let keying = RwSignal::new(KeyingStats::default());

  let next_target = move || {
    let text = match send_target.get() {
      SendTarget::Char => {
        let pool: Vec<&'static MorseChar> = LETTERS.iter().chain(DIGITS).collect();
        pool[random_index(pool.len())].ch.to_owned()
      }
      SendTarget::Word => {
        let pool: Vec<&'static MorseChar> = LETTERS.iter().chain(DIGITS).collect();
        let len = 2 + random_index(2); // 2–3 个字符
        (0..len)
          .map(|_| pool[random_index(pool.len())].ch)
          .collect()
      }
      SendTarget::CallSign => {
        // 官方个人业余电台呼号：B + 电台种类 A–H + 分区号 0–9 + 2 字母后缀
        const KIND: &[char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        let letters: Vec<&'static MorseChar> = LETTERS.iter().collect();
        let mut s = String::from("B");
        s.push(KIND[random_index(KIND.len())]);
        s.push(char::from_digit(random_index(10) as u32, 10).expect("digit 0-9"));
        for _ in 0..2 {
          s.push_str(letters[random_index(letters.len())].ch);
        }
        s
      }
    };
    target.set(text);
    marks.set(String::new());
    feedback.set(None);
    keying.set(KeyingStats::default());
  };

  let pressing = RwSignal::new(false);
  let on_down = move |_| {
    pressing.set(true);
    press_time.set(Some(js_sys::Date::now()));
    start_tone(settings.tone_hz(), settings.volume());
  };
  let on_up = move |_| {
    pressing.set(false);
    stop_tone();
    if let Some(t0) = press_time.get() {
      let dt = js_sys::Date::now() - t0;
      press_time.set(None);
      let is_dot = dt < 200.0;
      let mark = if is_dot { "." } else { "-" };
      marks.update(|m| m.push_str(mark));
      keying.update(|k| {
        if is_dot {
          k.dot_sum += dt;
          k.dot_sum2 += dt * dt;
          k.dot_n += 1;
        } else {
          k.dash_sum += dt;
          k.dash_n += 1;
        }
      });
    }
  };

  let judge = move || {
    let m = marks.get();
    let expected: String = target.get().chars().filter_map(code_of).collect();
    let ok = !m.is_empty() && m == expected;
    if ok {
      let new_streak = streak.get_untracked() + 1;
      streak.set(new_streak);
      stats.update(|s| {
        s.correct += 1;
        s.best_streak = s.best_streak.max(new_streak);
      });
    } else {
      streak.set(0);
      stats.update(|s| s.wrong += 1);
    }
    save_send(&stats.get_untracked());
    feedback.set(Some(ok));
  };

  let clear_marks = move || {
    marks.set(String::new());
    feedback.set(None);
    keying.set(KeyingStats::default());
  };

  let reset_stats = move || {
    let empty = SendStats::default();
    stats.set(empty.clone());
    streak.set(0);
    save_send(&empty);
  };

  let rate = move || {
    let s = stats.get();
    let total = s.correct + s.wrong;
    if total == 0 {
      0.0
    } else {
      s.correct as f64 / total as f64 * 100.0
    }
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("morse.sending-practice")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.hold-to-key-with")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-wrap items-center gap-1.5">
          <span class="mr-1 text-xs text-muted-foreground">{move || t("morse.target-type")}</span>
          {SendTarget::ALL
            .iter()
            .map(|&target_kind| {
              view! {
                <button
                  type="button"
                  on:click=move |_| {
                    send_target.set(target_kind);
                    next_target();
                  }
                  aria-pressed=move || (send_target.get() == target_kind).to_string()
                  class=move || pill_class(send_target.get() == target_kind)
                >
                  {move || t(target_kind.label())}
                </button>
              }
            })
            .collect_view()}
        </div>

        <div class="flex flex-col items-center gap-2 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("morse.target")}</div>
          <div class="text-4xl font-semibold tabular-nums tracking-widest">{move || target.get()}</div>
          <div data-testid="send-marks" class="font-mono text-2xl font-semibold tracking-[0.3em] text-primary">
            {move || {
              let m = marks.get();
              if m.is_empty() { "…".to_owned() } else { morse_display(&m) }
            }}
          </div>
          <div class="text-xs text-muted-foreground">{move || t("morse.short-press-0-2")}</div>
        </div>

        <div class="flex flex-col items-center gap-3">
          <button
            type="button"
            on:pointerdown=on_down
            on:pointerup=on_up
            on:pointerleave=on_up
            on:pointercancel=on_up
            style="touch-action: none;"
            class=move || {
              format!(
                "relative flex h-28 w-28 select-none items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg shadow-primary/25 transition-transform duration-150 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background {}",
                if pressing.get() { "scale-90" } else { "hover:scale-105" }
              )
            }
          >
            {move || {
              pressing.get().then(|| {
                view! { <span class="absolute inset-0 rounded-full bg-primary/40 animate-ping"></span> }
              })
            }}
            <span class="relative text-sm font-semibold">{move || t("morse.hold-to-send")}</span>
          </button>

          <div class="text-xs text-muted-foreground">
            {move || {
              let k = keying.get();
              let wpm = k.wpm();
              let score = k.rhythm_score();
              match (wpm, score) {
                (Some(w), Some(sc)) => view! {
                  <span>
                    {move || t("morse.estimated")} <span class="font-semibold tabular-nums text-foreground">{format!("{w:.0}")}</span> " WPM"
                    {move || t("morse.rhythm")} <span class="font-semibold tabular-nums text-foreground">{format!("{sc:.0}")}</span> "/100"
                  </span>
                }.into_any(),
                (Some(w), None) => view! {
                  <span>
                    {move || t("morse.estimated")} <span class="font-semibold tabular-nums text-foreground">{format!("{w:.0}")}</span> " WPM"
                  </span>
                }.into_any(),
                _ => view! { {move || t("morse.sending-estimates-your-speed")} }.into_any(),
              }
            }}
          </div>

          <div class="flex flex-wrap items-center justify-center gap-2">
            <Button
              variant=Variant::Default
              class="rounded-lg h-10"
              disabled=Signal::derive(move || marks.get().is_empty())
              on_click=Callback::new(move |_| judge())
            >
              {move || t("morse.judgement")}
            </Button>
            <Button
              variant=Variant::Outline
              class="rounded-lg h-10"
              on_click=Callback::new(move |_| clear_marks())
            >
              {move || t("morse.re-key")}
            </Button>
            <Button
              variant=Variant::Outline
              class="rounded-lg h-10"
              on_click=Callback::new(move |_| next_target())
            >
              {move || t("exam.next")}
            </Button>
          </div>

          <div class="min-h-5 text-sm" aria-live="polite">
            {move || {
              feedback.get().map(|ok| {
                if ok {
                  view! {
                    <span class="inline-flex items-center gap-1.5 font-medium text-emerald-600 dark:text-emerald-400 animate-in fade-in zoom-in-95 duration-200">
                      <Icon kind=IconKind::CheckCircle2 class="h-4 w-4" />
                      {move || t("learning.correct")}
                    </span>
                  }
                  .into_any()
                } else {
                  let expected: String = target.get().chars().filter_map(code_of).collect();
                  view! {
                    <span class="inline-flex items-center gap-1.5 font-medium text-red-600 dark:text-red-400 animate-in fade-in zoom-in-95 duration-200">
                      <Icon kind=IconKind::XCircle class="h-4 w-4" />
                      {move || t("morse.the-answer-is")} <span class="font-mono">{target.get()}</span> "　"
                      <span class="font-mono">{morse_display(&expected)}</span>
                    </span>
                  }
                  .into_any()
                }
              })
            }}
          </div>

          <div class="flex flex-wrap items-center justify-center gap-3 text-xs text-muted-foreground">
            <span>
              {move || t("common.correct-3")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().correct}</span>
            </span>
            <span>
              {move || t("morse.wrong")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().wrong}</span>
            </span>
            <span>
              {move || t("morse.accuracy")} <span class="font-semibold tabular-nums text-foreground">{move || format!("{:.0}%", rate())}</span>
            </span>
            <span>
              {move || t("morse.streak")} <span class="font-semibold tabular-nums text-foreground">{move || streak.get()}</span>
            </span>
            <span>
              {move || t("morse.best-streak")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().best_streak}</span>
            </span>
            <button
              type="button"
              on:click=move |_| reset_stats()
              class="rounded-md px-2 py-0.5 transition-all duration-200 hover:bg-accent hover:text-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              {move || t("morse.reset")}
            </button>
          </div>
        </div>
      </div>
    </section>
  }
}
