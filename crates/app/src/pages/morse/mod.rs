//! 莫尔斯电码速查表：字母（附语音字母）、数字、常用标点的点划序列与信号时值标准；
//! 并内置「点击试听」与「解码练习」两个交互能力。

mod abbrev_quiz;
mod callsign_runner;
mod callsign_session;
mod cw_decoder;
mod koch_trainer;
mod morse_card;
mod morse_key;
mod morse_page;
mod morse_trainer;
mod pill_button;
mod send_trainer;
mod signal_bars;
mod stats_panel;
mod word_copy;

pub use morse_page::MorsePage;
pub use pill_button::PillButton;

use ham_web_core::morse::code_of;
use ham_web_core::morse_store::{MorseStats, TrainerSettings};

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

/// 呼号抄收页里「静态标签」的胶囊样式。
///
/// 曾经带一个 `active` 参数、还有一支主色底 + `shadow-primary/20` 的分支，但唯一的调用点
/// （`callsign_runner.rs`）一直传 `false` 且挂在 `<span>` 上 —— 那一支是死代码，已删。
fn pill_class() -> &'static str {
  "inline-flex items-center justify-center whitespace-nowrap border bg-card px-3 py-1 text-xs"
}
