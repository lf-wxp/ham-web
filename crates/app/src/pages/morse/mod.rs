//! 莫尔斯电码速查表：字母（附语音字母）、数字、常用标点的点划序列与信号时值标准；
//! 并内置「点击试听」与「解码练习」两个交互能力。

mod morse_card;
mod morse_page;
mod morse_trainer;
mod send_trainer;

pub use morse_page::MorsePage;

use std::collections::HashMap;

use ham_web_core::morse::{COMMON_WORDS, DIGITS, LETTERS, MorseChar, PUNCTUATION, code_of};
use serde::{Deserialize, Serialize};

use crate::util::random;

/// 卡片试听的默认速度。
const CARD_WPM: f64 = 20.0;

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

/// 练习模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
  /// 听译：播放音频，靠耳朵识别字符。
  Listen,
  /// 看码：显示点划，识别对应字符。
  Read,
}

impl Mode {
  const fn label(self) -> &'static str {
    match self {
      Self::Listen => "听译",
      Self::Read => "看码",
    }
  }
}

const MODES: [Mode; 2] = [Mode::Listen, Mode::Read];

/// 出题范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
  Letters,
  Digits,
  Punctuation,
  Both,
  All,
}

impl Scope {
  const fn label(self) -> &'static str {
    match self {
      Self::Letters => "字母",
      Self::Digits => "数字",
      Self::Punctuation => "标点",
      Self::Both => "字母 + 数字",
      Self::All => "全部",
    }
  }

  const ALL: [Self; 5] = [
    Self::Letters,
    Self::Digits,
    Self::Punctuation,
    Self::Both,
    Self::All,
  ];
}

/// 题目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
  /// 单字符。
  Single,
  /// 单词（多个字母数字）。
  Word,
  /// 呼号（中国业余电台格式，如 BG4XXX）。
  CallSign,
  /// 整句（多个通联常用词）。
  Sentence,
}

impl Target {
  const fn label(self) -> &'static str {
    match self {
      Self::Single => "单字符",
      Self::Word => "单词",
      Self::CallSign => "呼号",
      Self::Sentence => "整句",
    }
  }

  /// 输入框的最大长度。
  const fn max_len(self) -> &'static str {
    match self {
      Self::Single => "1",
      Self::Word | Self::CallSign => "6",
      Self::Sentence => "20",
    }
  }

  /// 输入框占位提示。
  const fn placeholder(self) -> &'static str {
    match self {
      Self::Single => "字符",
      Self::Word => "单词",
      Self::CallSign => "呼号",
      Self::Sentence => "整句",
    }
  }

  const ALL: [Self; 4] = [Self::Single, Self::Word, Self::CallSign, Self::Sentence];
}

/// 一道题：答案文本 + 点划序列（空格分隔字符）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Question {
  text: String,
  code: String,
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
}

const STATS_KEY: &str = "morse-stats";

fn load_stats() -> MorseStats {
  crate::util::storage::get_json(STATS_KEY).unwrap_or_default()
}

fn save_stats(stats: &MorseStats) {
  crate::util::storage::set_json(STATS_KEY, stats);
}

fn random_index(len: usize) -> usize {
  (random() * len as f64) as usize
}

/// 指定范围的字符池。
fn scope_pool(scope: Scope) -> Vec<&'static MorseChar> {
  match scope {
    Scope::Letters => LETTERS.iter().collect(),
    Scope::Digits => DIGITS.iter().collect(),
    Scope::Punctuation => PUNCTUATION.iter().collect(),
    Scope::Both => LETTERS.iter().chain(DIGITS).collect(),
    Scope::All => LETTERS.iter().chain(DIGITS).chain(PUNCTUATION).collect(),
  }
}

/// 随机取一个字符；`weak_first` 为真且存在答错 ≥2 次的字符时，优先从易错字符中抽。
fn weighted_char(
  pool: &[&'static MorseChar],
  mistakes: &HashMap<String, usize>,
  weak_first: bool,
) -> &'static MorseChar {
  if weak_first {
    let frequent: Vec<&'static MorseChar> = pool
      .iter()
      .copied()
      .filter(|c| mistakes.get(c.ch).copied().unwrap_or(0) >= 2)
      .collect();
    if !frequent.is_empty() {
      return frequent[random_index(frequent.len())];
    }
  }
  pool[random_index(pool.len())]
}

/// 把文本编码为点划序列（字符之间用空格分隔）。
fn encode_text(text: &str) -> String {
  text
    .chars()
    .filter_map(code_of)
    .collect::<Vec<_>>()
    .join(" ")
}

/// 随机生成一道题。
fn random_question(
  scope: Scope,
  target: Target,
  mistakes: &HashMap<String, usize>,
  weak_first: bool,
) -> Question {
  match target {
    Target::Single => {
      let pool = scope_pool(scope);
      let c = weighted_char(&pool, mistakes, weak_first);
      Question {
        text: c.ch.to_owned(),
        code: c.code.to_owned(),
      }
    }
    Target::Word => {
      let pool: Vec<&'static MorseChar> = LETTERS.iter().chain(DIGITS).collect();
      let len = 2 + random_index(3); // 2–4 个字符
      let mut text = String::new();
      let mut codes = Vec::new();
      for _ in 0..len {
        let c = weighted_char(&pool, mistakes, weak_first);
        text.push_str(c.ch);
        codes.push(c.code);
      }
      Question {
        text,
        code: codes.join(" "),
      }
    }
    Target::CallSign => {
      // 官方个人业余电台呼号：前缀 B + 电台种类 A–H + 分区号 0–9 + 2–3 字母后缀
      const KIND: &[char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
      let letters: Vec<&'static MorseChar> = LETTERS.iter().collect();
      let suffix_len = 2 + random_index(2); // 2–3 个字母
      let mut suffix = String::new();
      for _ in 0..suffix_len {
        suffix.push_str(weighted_char(&letters, mistakes, weak_first).ch);
      }
      // 规避与遇险信号冲突的后缀 SOS
      if suffix == "SOS" {
        suffix = "SOX".to_owned();
      }
      let mut text = String::from("B");
      text.push(KIND[random_index(KIND.len())]);
      text.push(char::from_digit(random_index(10) as u32, 10).expect("digit 0-9"));
      text.push_str(&suffix);
      Question {
        code: encode_text(&text),
        text,
      }
    }
    Target::Sentence => {
      // 2–4 个通联常用词组成的「整句」
      let len = 2 + random_index(3);
      let mut words = Vec::new();
      for _ in 0..len {
        words.push(COMMON_WORDS[random_index(COMMON_WORDS.len())]);
      }
      let text = words.join(" ");
      Question {
        code: encode_text(&text),
        text,
      }
    }
  }
}

fn pill_class(active: bool) -> &'static str {
  if active {
    "rounded-full border bg-primary text-primary-foreground px-3 py-1 text-xs transition-colors"
  } else {
    "rounded-full border px-3 py-1 text-xs transition-colors hover:bg-accent"
  }
}
