//! Koch 法摩尔斯抄收训练：字符引入顺序、Farnsworth 时值、分组评分与升级规则。
//!
//! Koch 法以目标字符速度（通常 ≥ 15 WPM）从 2 个字符起步，抄收正确率达到 [`PASS_RATE`] 后再加入
//! 下一个字符；Farnsworth 法保持字符本身的速度，只拉长字符与单词之间的间隔，降低初学难度。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 字符引入顺序（与 LCWO 一致，共 41 个）。
pub const KOCH_ORDER: &str = "KMURESNAPTLWI.JZ=FOY,VG5/Q92H38B?47C1D60X";
/// 起步字符数。
pub const START_LEVEL: usize = 2;
/// 升级所需正确率。
pub const PASS_RATE: f64 = 0.9;
/// 判定升级至少需要抄收的字符数（当前级别内）。
pub const MIN_CHARS: u32 = 50;

/// 全部级别数。
#[must_use]
pub fn max_level() -> usize {
  KOCH_ORDER.chars().count()
}

/// 某级别已学的字符。
#[must_use]
pub fn chars_at(level: usize) -> Vec<char> {
  KOCH_ORDER
    .chars()
    .take(level.clamp(START_LEVEL, max_level()))
    .collect()
}

/// 一次播放的时值（秒）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
  /// 点长。
  pub dot: f64,
  /// 字符间隔。
  pub char_gap: f64,
  /// 单词间隔。
  pub word_gap: f64,
}

/// Farnsworth 时值（ARRL 公式）：字符按 `char_wpm` 发送，整体平均速度为 `effective_wpm`。
/// `effective_wpm >= char_wpm` 时为标准时值（字符间隔 3 点、单词间隔 7 点）。
#[must_use]
pub fn farnsworth(char_wpm: f64, effective_wpm: f64) -> Timing {
  let c = char_wpm.max(1.0);
  let dot = 1.2 / c;
  let s = effective_wpm.clamp(1.0, c);
  if s >= c {
    return Timing {
      dot,
      char_gap: 3.0 * dot,
      word_gap: 7.0 * dot,
    };
  }
  let delay = (60.0 * c - 37.2 * s) / (s * c);
  Timing {
    dot,
    char_gap: 3.0 * delay / 19.0,
    word_gap: 7.0 * delay / 19.0,
  }
}

/// 单个字符的抄收统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharTally {
  pub sent: u32,
  pub correct: u32,
}

impl CharTally {
  /// 错误率（0–1），未出现过时为 `None`。
  #[must_use]
  pub fn error_rate(self) -> Option<f64> {
    (self.sent > 0).then(|| f64::from(self.sent - self.correct) / f64::from(self.sent))
  }
}

/// Koch 训练进度（`localStorage` 的 `morse-koch`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KochProgress {
  /// 已学字符数。
  pub level: usize,
  /// 字符速度（WPM）。
  pub char_wpm: u32,
  /// 有效速度（WPM，Farnsworth）。
  pub effective_wpm: u32,
  /// 每个字符的累计统计。
  #[serde(default)]
  pub chars: BTreeMap<char, CharTally>,
  /// 当前级别内的抄收统计（升级判定用）。
  #[serde(default)]
  pub level_tally: CharTally,
}

impl Default for KochProgress {
  fn default() -> Self {
    Self {
      level: START_LEVEL,
      char_wpm: 20,
      effective_wpm: 10,
      chars: BTreeMap::new(),
      level_tally: CharTally::default(),
    }
  }
}

/// 一组抄收的评分结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupResult {
  /// 逐字符 `(应为, 是否抄对)`。
  pub marks: Vec<(char, bool)>,
  /// 本组后是否升级。
  pub leveled_up: bool,
}

impl GroupResult {
  #[must_use]
  pub fn correct(&self) -> usize {
    self.marks.iter().filter(|(_, ok)| *ok).count()
  }
}

/// 按位置逐字符比对（忽略大小写），漏抄的位置记错。
///
/// 抄收的组数与原文一致时逐组比对，一组里漏抄不会影响后面的组；否则忽略空白整体比对。
#[must_use]
pub fn score(expected: &str, typed: &str) -> Vec<(char, bool)> {
  let upper = |s: &str| {
    s.chars()
      .map(|c| c.to_ascii_uppercase())
      .collect::<Vec<_>>()
  };
  let compare = |exp: &[char], got: &[char]| {
    exp
      .iter()
      .enumerate()
      .map(|(i, &c)| (c, got.get(i) == Some(&c)))
      .collect::<Vec<_>>()
  };
  let exp_groups: Vec<Vec<char>> = expected.split_whitespace().map(upper).collect();
  let got_groups: Vec<Vec<char>> = typed.split_whitespace().map(upper).collect();
  if exp_groups.len() == got_groups.len() {
    exp_groups
      .iter()
      .zip(&got_groups)
      .flat_map(|(e, g)| compare(e, g))
      .collect()
  } else {
    compare(&exp_groups.concat(), &got_groups.concat())
  }
}

impl KochProgress {
  /// 已学字符。
  #[must_use]
  pub fn chars(&self) -> Vec<char> {
    chars_at(self.level)
  }

  /// 最新加入的字符。
  #[must_use]
  pub fn newest(&self) -> Option<char> {
    self.chars().last().copied()
  }

  /// 当前级别正确率。
  #[must_use]
  pub fn level_rate(&self) -> Option<f64> {
    self.level_tally.error_rate().map(|e| 1.0 - e)
  }

  /// 出题权重：最新字符与错误率高的字符出现更多。
  #[must_use]
  pub fn weights(&self) -> Vec<(char, f64)> {
    let newest = self.newest();
    self
      .chars()
      .into_iter()
      .map(|c| {
        let err = self
          .chars
          .get(&c)
          .and_then(|t| t.error_rate())
          .unwrap_or(0.0);
        let bonus = if Some(c) == newest { 2.0 } else { 0.0 };
        (c, 1.0 + 3.0 * err + bonus)
      })
      .collect()
  }

  /// 生成 `groups` 组、每组 `len` 个字符的练习文本（组间空格分隔）；`rng` 返回 `[0, 1)`。
  pub fn generate(&self, groups: usize, len: usize, rng: &mut impl FnMut() -> f64) -> String {
    let weights = self.weights();
    let total: f64 = weights.iter().map(|(_, w)| w).sum();
    let mut pick = || {
      let mut x = rng() * total;
      for (c, w) in &weights {
        if x < *w {
          return *c;
        }
        x -= w;
      }
      weights.last().map_or('K', |(c, _)| *c)
    };
    (0..groups)
      .map(|_| (0..len).map(|_| pick()).collect::<String>())
      .collect::<Vec<_>>()
      .join(" ")
  }

  /// 记录一组抄收结果；当前级别抄收满 [`MIN_CHARS`] 个且正确率 ≥ [`PASS_RATE`] 时自动升级。
  pub fn record(&mut self, expected: &str, typed: &str) -> GroupResult {
    let marks = score(expected, typed);
    for &(c, ok) in &marks {
      let t = self.chars.entry(c).or_default();
      t.sent += 1;
      t.correct += u32::from(ok);
      self.level_tally.sent += 1;
      self.level_tally.correct += u32::from(ok);
    }
    let leveled_up = self.level < max_level()
      && self.level_tally.sent >= MIN_CHARS
      && self.level_rate().is_some_and(|r| r >= PASS_RATE);
    if leveled_up {
      self.set_level(self.level + 1);
    }
    GroupResult { marks, leveled_up }
  }

  /// 手动调整级别（清零当前级别统计）。
  pub fn set_level(&mut self, level: usize) {
    self.level = level.clamp(START_LEVEL, max_level());
    self.level_tally = CharTally::default();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn order_has_unique_chars() {
    let mut v: Vec<char> = KOCH_ORDER.chars().collect();
    let n = v.len();
    v.sort_unstable();
    v.dedup();
    assert_eq!((n, v.len()), (41, 41));
    assert!(v.iter().all(|&c| crate::morse::code_of(c).is_some()));
    assert_eq!(chars_at(0), vec!['K', 'M']);
    assert_eq!(chars_at(100).len(), 41);
  }

  #[test]
  fn farnsworth_matches_arrl_formula() {
    let std = farnsworth(20.0, 20.0);
    assert!((std.dot - 0.06).abs() < 1e-9);
    assert!((std.char_gap - 0.18).abs() < 1e-9);
    let f = farnsworth(18.0, 5.0);
    // ARRL：18/5 WPM 时 ta = (60*18 - 37.2*5) / (5*18) ≈ 9.933 s。
    let ta = (60.0 * 18.0 - 37.2 * 5.0) / 90.0;
    assert!((f.char_gap - 3.0 * ta / 19.0).abs() < 1e-9);
    assert!(f.char_gap > 3.0 * f.dot && f.word_gap > f.char_gap);
    assert_eq!(farnsworth(10.0, 30.0), farnsworth(10.0, 10.0));
  }

  #[test]
  fn scores_by_position() {
    assert_eq!(
      score("KM MK", "km  m"),
      vec![('K', true), ('M', true), ('M', true), ('K', false)]
    );
    // 第一组漏抄一个字，不拖累第二组
    let marks = score("KMR MKR", "KR MKR");
    assert_eq!(marks.iter().filter(|m| m.1).count(), 4);
    assert!(marks[3..].iter().all(|m| m.1));
  }

  #[test]
  fn levels_up_after_enough_accurate_copy() {
    let mut p = KochProgress::default();
    let group = "KMKMKMKMKM KMKMKMKMKM";
    // 10/20 → 30/40（不足 50 字）→ 50/60 → 70/80（87.5%）均不升级，90/100 升级。
    let r = p.record(group, "KMKMKMKMKM");
    assert_eq!(r.correct(), 10);
    for _ in 0..3 {
      assert!(!p.record(group, group).leveled_up);
    }
    assert_eq!(p.level, 2);
    assert!(p.record(group, group).leveled_up);
    assert_eq!((p.level, p.level_tally.sent), (3, 0));
    assert_eq!(p.chars[&'M'].sent, 50);
    assert_eq!(p.newest(), Some('U'));
  }

  #[test]
  fn generate_uses_learned_chars_and_weights_weak_ones() {
    let mut p = KochProgress::default();
    p.set_level(5);
    p.chars.insert(
      'K',
      CharTally {
        sent: 10,
        correct: 0,
      },
    );
    let mut seed = 7u64;
    let mut rng = move || {
      seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
      (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let text = p.generate(20, 5, &mut rng);
    assert_eq!(text.split(' ').count(), 20);
    assert!(text.chars().all(|c| c == ' ' || "KMURE".contains(c)));
    let count = |ch| text.chars().filter(|&c| c == ch).count();
    assert!(count('K') > count('M'));
    let w = p.weights();
    assert!(w.iter().find(|(c, _)| *c == 'E').expect("newest").1 > 1.0);
  }

  #[test]
  fn progress_json_defaults() {
    let p: KochProgress =
      serde_json::from_str(r#"{"level":4,"char_wpm":18,"effective_wpm":8}"#).expect("json");
    assert_eq!(p.chars(), vec!['K', 'M', 'U', 'R']);
    assert!(p.chars.is_empty());
  }
}
