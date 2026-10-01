//! 摩尔斯出题：练习模式 / 范围 / 类型与题目生成（解码练习共用）。
//!
//! 不依赖浏览器，`rng` 注入以便原生单测。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cw_runner::random_call;
use crate::morse::{DIGITS, LETTERS, MorseChar, PUNCTUATION, code_of};

/// 练习模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
  /// 听译：播放音频，靠耳朵识别字符。
  Listen,
  /// 看码：显示点划，识别对应字符。
  Read,
  /// 选择：从四个选项中选出正确答案。
  Choice,
}

impl Mode {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Listen => "听译",
      Self::Read => "看码",
      Self::Choice => "选择",
    }
  }
}

pub const MODES: [Mode; 3] = [Mode::Listen, Mode::Read, Mode::Choice];

/// 出题范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
  Letters,
  Digits,
  Punctuation,
  Both,
  All,
  /// 易错：只练答错过的字符（单字符）。
  Weak,
}

impl Scope {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Letters => "字母",
      Self::Digits => "数字",
      Self::Punctuation => "标点",
      Self::Both => "字母 + 数字",
      Self::All => "全部",
      Self::Weak => "易错",
    }
  }

  pub const ALL: [Self; 6] = [
    Self::Letters,
    Self::Digits,
    Self::Punctuation,
    Self::Both,
    Self::All,
    Self::Weak,
  ];
}

/// 题目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
  /// 单字符。
  Single,
  /// 单词（多个字母数字）。
  Word,
  /// 呼号（国际真实呼号，如 DL1ABC）。
  CallSign,
  /// 整句（真实通联句式）。
  Sentence,
}

impl Target {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Single => "单字符",
      Self::Word => "单词",
      Self::CallSign => "呼号",
      Self::Sentence => "整句",
    }
  }

  /// 输入框的最大长度。
  pub const fn max_len(self) -> &'static str {
    match self {
      Self::Single => "1",
      Self::Word | Self::CallSign => "6",
      Self::Sentence => "50",
    }
  }

  /// 输入框占位提示。
  pub const fn placeholder(self) -> &'static str {
    match self {
      Self::Single => "字符",
      Self::Word => "单词",
      Self::CallSign => "呼号",
      Self::Sentence => "整句",
    }
  }

  pub const ALL: [Self; 4] = [Self::Single, Self::Word, Self::CallSign, Self::Sentence];
}

/// 一道题：答案文本 + 点划序列（空格分隔字符）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
  pub text: String,
  pub code: String,
}

fn idx(len: usize, rng: &mut impl FnMut() -> f64) -> usize {
  ((rng() * len as f64) as usize).min(len.saturating_sub(1))
}

/// 指定范围的字符池。
fn scope_pool(scope: Scope) -> Vec<&'static MorseChar> {
  match scope {
    Scope::Letters => LETTERS.iter().collect(),
    Scope::Digits => DIGITS.iter().collect(),
    Scope::Punctuation => PUNCTUATION.iter().collect(),
    Scope::Both => LETTERS.iter().chain(DIGITS).collect(),
    Scope::All => LETTERS.iter().chain(DIGITS).chain(PUNCTUATION).collect(),
    Scope::Weak => LETTERS.iter().chain(DIGITS).chain(PUNCTUATION).collect(),
  }
}

/// 易错字符池（答错 ≥1 次的字符）。
fn weak_pool(mistakes: &HashMap<String, usize>) -> Vec<&'static MorseChar> {
  LETTERS
    .iter()
    .chain(DIGITS)
    .chain(PUNCTUATION)
    .filter(|c| mistakes.get(c.ch).copied().unwrap_or(0) >= 1)
    .collect()
}

/// 随机取一个字符；`weak_first` 为真且存在答错 ≥2 次的字符时，优先从易错字符中抽。
fn weighted_char(
  pool: &[&'static MorseChar],
  mistakes: &HashMap<String, usize>,
  weak_first: bool,
  rng: &mut impl FnMut() -> f64,
) -> &'static MorseChar {
  if weak_first {
    let frequent: Vec<&'static MorseChar> = pool
      .iter()
      .copied()
      .filter(|c| mistakes.get(c.ch).copied().unwrap_or(0) >= 2)
      .collect();
    if !frequent.is_empty() {
      return frequent[idx(frequent.len(), rng)];
    }
  }
  pool[idx(pool.len(), rng)]
}

/// 把文本编码为点划序列（字符之间用空格分隔）。
pub fn encode_text(text: &str) -> String {
  text
    .chars()
    .filter_map(code_of)
    .collect::<Vec<_>>()
    .join(" ")
}

/// 生成一句真实感通联（复用真实呼号与常见缩语，而非随机单词堆叠）。
fn random_sentence(rng: &mut impl FnMut() -> f64) -> String {
  let call = random_call(rng);
  const TEMPLATES: [&str; 8] = [
    "CQ CQ DE CALL K",
    "CALL DE BG4XYZ K",
    "TNX CALL UR 5NN QSL K",
    "R R CALL 73 SK",
    "CALL UR RST 599 K",
    "QTH BEIJING NAME TOM",
    "DE BG4XYZ UR RST 579 K",
    "CALL PSE K",
  ];
  TEMPLATES[idx(TEMPLATES.len(), rng)].replace("CALL", &call)
}

/// 随机生成一道题。
pub fn random_question(
  scope: Scope,
  target: Target,
  mistakes: &HashMap<String, usize>,
  weak_first: bool,
  rng: &mut impl FnMut() -> f64,
) -> Question {
  // 易错范围：只出答错过的单字符（无易错字符时退回全部单字符）
  if scope == Scope::Weak {
    let weak = weak_pool(mistakes);
    let pool = if weak.is_empty() {
      scope_pool(Scope::All)
    } else {
      weak
    };
    let c = pool[idx(pool.len(), rng)];
    return Question {
      text: c.ch.to_owned(),
      code: c.code.to_owned(),
    };
  }
  match target {
    Target::Single => {
      let pool = scope_pool(scope);
      let c = weighted_char(&pool, mistakes, weak_first, rng);
      Question {
        text: c.ch.to_owned(),
        code: c.code.to_owned(),
      }
    }
    Target::Word => {
      let pool: Vec<&'static MorseChar> = LETTERS.iter().chain(DIGITS).collect();
      let len = 2 + idx(3, rng); // 2–4 个字符
      let mut text = String::new();
      let mut codes = Vec::new();
      for _ in 0..len {
        let c = weighted_char(&pool, mistakes, weak_first, rng);
        text.push_str(c.ch);
        codes.push(c.code);
      }
      Question {
        text,
        code: codes.join(" "),
      }
    }
    Target::CallSign => {
      // 国际真实呼号（复用竞赛模拟的生成逻辑，含常见前缀）
      let text = random_call(rng);
      Question {
        code: encode_text(&text),
        text,
      }
    }
    Target::Sentence => {
      let text = random_sentence(rng);
      Question {
        code: encode_text(&text),
        text,
      }
    }
  }
}

/// 生成一道选择题：正确答案 + 3 个同类干扰项，返回打乱后的选项与正确下标。
pub fn random_choice(
  scope: Scope,
  target: Target,
  mistakes: &HashMap<String, usize>,
  weak_first: bool,
  rng: &mut impl FnMut() -> f64,
) -> (Question, Vec<String>, usize) {
  let question = random_question(scope, target, mistakes, weak_first, rng);
  // 易错范围：干扰项从全量单字符池抽取，保证能凑齐 4 个不同选项
  let (d_scope, d_target) = if scope == Scope::Weak {
    (Scope::All, Target::Single)
  } else {
    (scope, target)
  };
  let mut options: Vec<String> = vec![question.text.clone()];
  let mut guard = 0;
  while options.len() < 4 && guard < 100 {
    guard += 1;
    // 干扰项不加权（false），避免在易错/加权下重复抽到同一个字符
    let d = random_question(d_scope, d_target, mistakes, false, rng).text;
    if !options.contains(&d) {
      options.push(d);
    }
  }
  let mut shuffled = options;
  for i in (1..shuffled.len()).rev() {
    let j = idx(i + 1, rng);
    shuffled.swap(i, j);
  }
  let answer_idx = shuffled
    .iter()
    .position(|o| *o == question.text)
    .unwrap_or(0);
  (question, shuffled, answer_idx)
}

/// 跨天归档每日统计：日期变化时把当日计数写入 `daily` 并清零。
pub fn roll_daily_snapshot(
  daily: &mut HashMap<String, (usize, usize)>,
  today: &mut String,
  today_correct: &mut usize,
  today_wrong: &mut usize,
  new_today: &str,
) {
  if *today != new_today {
    if !today.is_empty() {
      let old = std::mem::take(today);
      daily.insert(old, (*today_correct, *today_wrong));
    }
    *today = new_today.to_owned();
    *today_correct = 0;
    *today_wrong = 0;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 线性同余伪随机，测试可复现。
  fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
      s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
      (s >> 11) as f64 / (1u64 << 53) as f64
    }
  }

  #[test]
  fn weak_scope_only_picks_mistaken_chars() {
    let mut rng = lcg(7);
    let mut mistakes = HashMap::new();
    mistakes.insert("Q".to_owned(), 3usize);
    mistakes.insert("Z".to_owned(), 1usize);
    for _ in 0..50 {
      let q = random_question(Scope::Weak, Target::Single, &mistakes, true, &mut rng);
      assert!(q.text == "Q" || q.text == "Z", "unexpected {}", q.text);
    }
  }

  #[test]
  fn choice_includes_correct_and_three_distinct_options() {
    let mut rng = lcg(3);
    for _ in 0..20 {
      let (q, opts, ans) = random_choice(
        Scope::Letters,
        Target::Single,
        &HashMap::new(),
        true,
        &mut rng,
      );
      assert_eq!(opts.len(), 4);
      assert!(opts.contains(&q.text));
      assert_eq!(opts[ans], q.text);
    }
  }

  #[test]
  fn callsign_and_sentence_encode_to_valid_morse() {
    let mut rng = lcg(11);
    for _ in 0..30 {
      let q = random_question(
        Scope::All,
        Target::CallSign,
        &HashMap::new(),
        false,
        &mut rng,
      );
      assert!(!q.text.is_empty() && !q.code.is_empty());
      assert!(q.code.chars().all(|c| c == '.' || c == '-' || c == ' '));
    }
    let q = random_question(
      Scope::All,
      Target::Sentence,
      &HashMap::new(),
      false,
      &mut rng,
    );
    assert!(q.text.contains(' '));
    assert!(q.code.chars().all(|c| c == '.' || c == '-' || c == ' '));
  }

  #[test]
  fn choice_weak_scope_still_has_four_options() {
    let mut rng = lcg(5);
    // 只有一个易错字符时，干扰项应从全量池补齐，仍凑齐 4 个选项
    let mut mistakes = HashMap::new();
    mistakes.insert("Q".to_owned(), 2usize);
    for _ in 0..20 {
      let (q, opts, ans) = random_choice(Scope::Weak, Target::Single, &mistakes, true, &mut rng);
      assert_eq!(opts.len(), 4, "options: {opts:?}");
      assert_eq!(q.text, "Q");
      assert!(opts.contains(&q.text));
      assert_eq!(opts[ans], q.text);
    }
  }

  #[test]
  fn roll_daily_snapshot_archives_on_day_change() {
    let mut daily = HashMap::new();
    let mut today = String::new();
    let mut ok = 0usize;
    let mut bad = 0usize;
    // 首日初始化
    roll_daily_snapshot(&mut daily, &mut today, &mut ok, &mut bad, "2026-10-01");
    assert_eq!(today, "2026-10-01");
    assert_eq!((ok, bad), (0, 0));
    // 当日练习
    ok = 5;
    bad = 1;
    // 同日再滚动：不归档
    roll_daily_snapshot(&mut daily, &mut today, &mut ok, &mut bad, "2026-10-01");
    assert!(daily.is_empty());
    assert_eq!((ok, bad), (5, 1));
    // 跨日：归档并清零
    roll_daily_snapshot(&mut daily, &mut today, &mut ok, &mut bad, "2026-10-02");
    assert_eq!(today, "2026-10-02");
    assert_eq!((ok, bad), (0, 0));
    assert_eq!(daily.get("2026-10-01"), Some(&(5, 1)));
  }
}
