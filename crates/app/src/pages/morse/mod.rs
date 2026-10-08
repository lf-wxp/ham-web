//! 莫尔斯电码速查表：字母（附语音字母）、数字、常用标点的点划序列与信号时值标准；
//! 并内置「点击试听」与「解码练习」两个交互能力。

mod abbrev_quiz;
mod callsign_runner;
mod callsign_session;
mod cw_decoder;
mod koch_trainer;
mod morse_card;
mod morse_page;
mod morse_trainer;
mod send_trainer;
mod signal_bars;
mod stats_panel;
mod word_copy;

pub use morse_page::MorsePage;

use std::collections::HashMap;

use ham_web_core::morse::code_of;
use serde::{Deserialize, Serialize};

use crate::util::random;

pub use ham_web_core::morse_trainer::{
  MODES, Mode, Question, Scope, Target, random_choice, random_question,
};

/// 卡片试听的默认速度。
const CARD_WPM: f64 = 20.0;

/// 文本 → 点划序列，空格分隔的单词之间插入单词间隔 `/`（供 `play_morse_timed` 播放）。
fn encode_words(text: &str) -> String {
  text
    .split_whitespace()
    .map(|w| w.chars().filter_map(code_of).collect::<Vec<_>>().join(" "))
    .collect::<Vec<_>>()
    .join(" / ")
}

/// 把点划序列渲染为更直观的视觉符号：`.` → `•`、`-` → `—`。
fn morse_display(code: &str) -> String {
  code
    .chars()
    .map(|c| {
      if c == '.' {
        '•'
      } else if c == ' ' {
        ' '
      } else {
        '—'
      }
    })
    .collect()
}

/// 提交反馈。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Feedback {
  ok: bool,
  question: Question,
}

/// 练习统计（持久化到 localStorage）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MorseStats {
  correct: usize,
  wrong: usize,
  /// 每个字符（答案文本）的累计答错次数，用于易错字符优先出题。
  mistakes: HashMap<String, usize>,
  /// 当前连续答对（不持久化，仅运行时）。
  #[serde(skip)]
  streak: usize,
  /// 历史最长连续答对。
  #[serde(default)]
  best_streak: usize,
  /// 答对过的最高速度（WPM）。
  #[serde(default)]
  top_wpm: f64,
  /// 今日日期（YYYY-MM-DD）。
  #[serde(default)]
  today: String,
  /// 今日正确/错误。
  #[serde(default)]
  today_correct: usize,
  #[serde(default)]
  today_wrong: usize,
  /// 每日快照（日期 → 正确/错误），用于近 7 天趋势。
  #[serde(default)]
  daily: HashMap<String, (usize, usize)>,
}

const STATS_KEY: &str = "morse-stats";

fn load_stats() -> MorseStats {
  crate::util::storage::get_json(STATS_KEY).unwrap_or_default()
}

fn save_stats(stats: &MorseStats) {
  crate::util::storage::set_json(STATS_KEY, stats);
}

/// 跨天后把昨日统计归档到 `daily` 并清零今日计数。
fn roll_today(stats: &mut MorseStats) {
  ham_web_core::morse_trainer::roll_daily_snapshot(
    &mut stats.daily,
    &mut stats.today,
    &mut stats.today_correct,
    &mut stats.today_wrong,
    &crate::util::local_today(),
  );
}

/// 导出全部摩尔斯练习统计（解码/Koch/呼号/发报）为单个 JSON 文件。
fn export_all_morse_stats() {
  let mut map = serde_json::Map::new();
  for key in ["morse-stats", "morse-koch", "morse-runner", "morse-send"] {
    if let Some(v) = crate::util::storage::get(key)
      && let Ok(j) = serde_json::from_str::<serde_json::Value>(&v)
    {
      map.insert(key.to_owned(), j);
    }
  }
  if let Ok(json) = serde_json::to_string_pretty(&serde_json::Value::Object(map)) {
    crate::util::download_text("morse-all-stats.json", &json, "application/json");
  }
}

/// 解码练习的设置（持久化）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct TrainerSettings {
  #[serde(default = "default_mode")]
  mode: Mode,
  #[serde(default = "default_scope")]
  scope: Scope,
  #[serde(default = "default_target")]
  target: Target,
  #[serde(default = "default_wpm")]
  wpm: f64,
  #[serde(default = "default_wpm")]
  eff_wpm: f64,
  /// 自适应速度：答对加速、答错减速。
  #[serde(default)]
  adaptive: bool,
}

const fn default_mode() -> Mode {
  Mode::Listen
}

const fn default_scope() -> Scope {
  Scope::Letters
}

const fn default_target() -> Target {
  Target::Single
}

const fn default_wpm() -> f64 {
  20.0
}

impl Default for TrainerSettings {
  fn default() -> Self {
    Self {
      mode: default_mode(),
      scope: default_scope(),
      target: default_target(),
      wpm: default_wpm(),
      eff_wpm: default_wpm(),
      adaptive: false,
    }
  }
}

const TRAINER_KEY: &str = "morse-trainer";

fn load_trainer_settings() -> TrainerSettings {
  crate::util::storage::get_json(TRAINER_KEY).unwrap_or_default()
}

fn save_trainer_settings(s: &TrainerSettings) {
  crate::util::storage::set_json(TRAINER_KEY, s);
}

fn random_index(len: usize) -> usize {
  (random() * len as f64) as usize
}

fn pill_class(active: bool) -> &'static str {
  if active {
    "inline-flex items-center justify-center whitespace-nowrap rounded-full border border-primary bg-primary px-3 py-1 text-xs font-medium text-primary-foreground shadow-sm shadow-primary/20 transition-all duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background"
  } else {
    "inline-flex items-center justify-center whitespace-nowrap rounded-full border bg-card px-3 py-1 text-xs transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background"
  }
}
