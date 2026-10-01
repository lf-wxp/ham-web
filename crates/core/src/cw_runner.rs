//! CW 呼号抄收训练（简化版 Morse Runner）：随机生成真实前缀的呼号与竞赛交换（5NN + 序号），
//! 抄收后计分，并可按表现自动调整速度。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 常见前缀（按出现频率粗略加权：重复出现即权重更高）。以数字结尾的前缀不再追加区号。
const PREFIXES: &[&str] = &[
  "K", "K", "K", "W", "W", "W", "N", "N", "AA", "AB", "KD", "KB", "WA", "VE", "VA", "XE", "KP4",
  "KH6", "KL7", "DL", "DL", "DK", "DJ", "G", "M", "2E", "F", "I", "IK", "EA", "CT", "PA", "ON",
  "OZ", "SM", "LA", "OH", "SP", "SP", "OK", "OM", "HA", "S5", "9A", "YU", "LZ", "YO", "UR", "UA",
  "UA", "RA", "RW", "LY", "YL", "ES", "EU", "SV", "4X", "TA", "JA", "JA", "JH", "JR", "7K", "HL",
  "DS", "BG", "BG", "BH", "BD", "BA", "BV", "VR2", "VU", "HS", "9M2", "YB", "DU", "VK", "VK", "ZL",
  "PY", "PY", "LU", "CE", "CX", "HK", "OA", "YV", "ZS", "CN", "5B4", "EA8", "CT3",
];

/// 序号中可截短的数字（竞赛常用：0 → T，9 → N）。
fn cut(c: char) -> char {
  match c {
    '0' => 'T',
    '9' => 'N',
    other => other,
  }
}

fn uncut(c: char) -> char {
  match c.to_ascii_uppercase() {
    'T' | 'O' => '0',
    'N' => '9',
    other => other,
  }
}

/// 按 `[0, 1)` 随机数取下标。
fn pick(len: usize, rng: &mut impl FnMut() -> f64) -> usize {
  ((rng() * len as f64) as usize).min(len.saturating_sub(1))
}

/// 随机生成一个呼号，如 `DL3ABC`、`KP4XY`、`BG4ZZ`。
pub fn random_call(rng: &mut impl FnMut() -> f64) -> String {
  let prefix = PREFIXES[pick(PREFIXES.len(), rng)];
  let mut call = prefix.to_owned();
  if !prefix.ends_with(|c: char| c.is_ascii_digit()) {
    call.push(char::from(b'0' + pick(10, rng) as u8));
  }
  // 后缀 1–3 个字母，2–3 位居多
  let len = [1, 2, 2, 3, 3, 3][pick(6, rng)];
  for _ in 0..len {
    call.push(char::from(b'A' + pick(26, rng) as u8));
  }
  call
}

/// 一次通联中对方发来的内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
  pub call: String,
  pub serial: u32,
}

impl Exchange {
  pub fn random(qso_no: u32, rng: &mut impl FnMut() -> f64) -> Self {
    // 对方的序号随比赛进程增长，带随机起伏
    let base = 1 + qso_no * 7 + pick(150, rng) as u32;
    Self {
      call: random_call(rng),
      serial: base,
    }
  }

  /// 播放用文本：`呼号 5NN 序号`，`cut_numbers` 时序号用截短数字。
  #[must_use]
  pub fn text(&self, cut_numbers: bool) -> String {
    let serial: String = if cut_numbers {
      format!("{:03}", self.serial).chars().map(cut).collect()
    } else {
      self.serial.to_string()
    };
    format!("{} 5NN {serial}", self.call)
  }
}

/// 一次通联的抄收结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QsoResult {
  pub call_ok: bool,
  pub serial_ok: bool,
  pub points: u32,
  /// 呼号中抄错 / 漏抄的字符（按原文位置）。
  pub missed: Vec<char>,
}

/// 呼号正确得分。
pub const CALL_POINTS: u32 = 2;
/// 序号正确得分。
pub const SERIAL_POINTS: u32 = 1;

/// 核对抄收内容。序号允许前导零与截短数字（T/O = 0，N = 9）。
#[must_use]
pub fn check(ex: &Exchange, typed_call: &str, typed_serial: &str) -> QsoResult {
  let typed: Vec<char> = typed_call
    .trim()
    .chars()
    .map(|c| c.to_ascii_uppercase())
    .collect();
  let expected: Vec<char> = ex.call.chars().collect();
  let missed = unmatched(&expected, &typed);
  let call_ok = typed == expected;
  let serial: String = typed_serial.trim().chars().map(uncut).collect();
  let serial_ok = serial.parse::<u32>().ok() == Some(ex.serial);
  let points = if call_ok { CALL_POINTS } else { 0 }
    + if call_ok && serial_ok {
      SERIAL_POINTS
    } else {
      0
    };
  QsoResult {
    call_ok,
    serial_ok,
    points,
    missed,
  }
}

/// `expected` 中没能与 `typed` 对齐（最长公共子序列之外）的字符：漏抄一个字不会把后面全算错。
fn unmatched(expected: &[char], typed: &[char]) -> Vec<char> {
  let (n, m) = (expected.len(), typed.len());
  let mut lcs = vec![vec![0u8; m + 1]; n + 1];
  for i in (0..n).rev() {
    for j in (0..m).rev() {
      lcs[i][j] = if expected[i] == typed[j] {
        lcs[i + 1][j + 1] + 1
      } else {
        lcs[i + 1][j].max(lcs[i][j + 1])
      };
    }
  }
  let (mut i, mut j) = (0, 0);
  let mut out = Vec::new();
  while i < n {
    if j < m && expected[i] == typed[j] {
      i += 1;
      j += 1;
    } else if j < m && lcs[i][j + 1] >= lcs[i + 1][j] {
      j += 1;
    } else {
      out.push(expected[i]);
      i += 1;
    }
  }
  out
}

pub const MIN_WPM: u32 = 10;
pub const MAX_WPM: u32 = 50;

/// 自适应速度：一次抄对且没重听则加速，抄错则减速。
#[must_use]
pub fn adapt_wpm(wpm: u32, result: &QsoResult, replays: u32) -> u32 {
  if result.call_ok && result.serial_ok && replays == 0 {
    (wpm + 1).min(MAX_WPM)
  } else if !result.call_ok {
    wpm.saturating_sub(2).max(MIN_WPM)
  } else {
    wpm
  }
}

/// 累计成绩（`localStorage` 中 `morse-runner`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerStats {
  #[serde(default = "default_wpm")]
  pub wpm: u32,
  #[serde(default = "default_true")]
  pub adaptive: bool,
  #[serde(default)]
  pub cut_numbers: bool,
  /// 叠听模式：每个通联额外发一个干扰台。
  #[serde(default)]
  pub pileup: bool,
  #[serde(default)]
  pub sessions: u32,
  #[serde(default)]
  pub qsos: u32,
  #[serde(default)]
  pub calls_ok: u32,
  /// 单轮最高分。
  #[serde(default)]
  pub best_score: u32,
  /// 抄对过的最高速度。
  #[serde(default)]
  pub top_wpm: u32,
  /// 呼号中各字符抄错的次数。
  #[serde(default)]
  pub missed: BTreeMap<char, u32>,
}

fn default_wpm() -> u32 {
  20
}

fn default_true() -> bool {
  true
}

impl Default for RunnerStats {
  fn default() -> Self {
    Self {
      wpm: default_wpm(),
      adaptive: true,
      cut_numbers: false,
      pileup: false,
      sessions: 0,
      qsos: 0,
      calls_ok: 0,
      best_score: 0,
      top_wpm: 0,
      missed: BTreeMap::new(),
    }
  }
}

impl RunnerStats {
  /// 记一次通联（`wpm` 为本次播放速度）。
  pub fn record(&mut self, result: &QsoResult, wpm: u32) {
    self.qsos += 1;
    if result.call_ok {
      self.calls_ok += 1;
      self.top_wpm = self.top_wpm.max(wpm);
    }
    for &c in &result.missed {
      *self.missed.entry(c).or_default() += 1;
    }
  }

  /// 一轮结束。
  pub fn finish_session(&mut self, score: u32) {
    self.sessions += 1;
    self.best_score = self.best_score.max(score);
  }

  /// 最常抄错的字符（次数降序）。
  #[must_use]
  pub fn worst_chars(&self, n: usize) -> Vec<(char, u32)> {
    let mut v: Vec<(char, u32)> = self.missed.iter().map(|(&c, &n)| (c, n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::dxcc;
  use crate::morse::code_of;

  /// 线性同余伪随机，测试可复现。
  fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
      s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
      (s >> 11) as f64 / (1u64 << 53) as f64
    }
  }

  #[test]
  fn random_calls_are_real_and_sendable() {
    let mut rng = lcg(7);
    for _ in 0..500 {
      let call = random_call(&mut rng);
      assert!((3..=7).contains(&call.len()), "{call}");
      assert!(call.chars().any(|c| c.is_ascii_digit()), "{call}");
      assert!(dxcc::lookup(&call).is_some(), "无法识别实体：{call}");
      assert!(call.chars().all(|c| code_of(c).is_some()), "{call}");
    }
  }

  #[test]
  fn exchange_text_and_cut_numbers() {
    let ex = Exchange {
      call: "DL1ABC".into(),
      serial: 90,
    };
    assert_eq!(ex.text(false), "DL1ABC 5NN 90");
    assert_eq!(ex.text(true), "DL1ABC 5NN TNT");
  }

  #[test]
  fn checks_call_and_serial() {
    let ex = Exchange {
      call: "JA1XYZ".into(),
      serial: 109,
    };
    let r = check(&ex, "ja1xyz", "1t9");
    assert!(r.call_ok && r.serial_ok);
    assert_eq!(r.points, CALL_POINTS + SERIAL_POINTS);
    let r = check(&ex, "JA1XY", "109");
    assert!(!r.call_ok);
    assert_eq!(r.points, 0);
    assert_eq!(r.missed, vec!['Z']);
    let r = check(&ex, "JA1XYZ", "190");
    assert_eq!(
      (r.call_ok, r.serial_ok, r.points),
      (true, false, CALL_POINTS)
    );
    // 多抄字符也不算对，但不记漏抄
    let r = check(&ex, "JA1XYZZ", "109");
    assert!(!r.call_ok && r.missed.is_empty());
    // 开头漏一个字只记这一个
    assert_eq!(check(&ex, "A1XYZ", "109").missed, vec!['J']);
    assert_eq!(check(&ex, "JA1QYZ", "109").missed, vec!['X']);
  }

  #[test]
  fn adapts_speed_and_tracks_stats() {
    let ok = QsoResult {
      call_ok: true,
      serial_ok: true,
      points: 3,
      missed: vec![],
    };
    let bad = QsoResult {
      call_ok: false,
      serial_ok: false,
      points: 0,
      missed: vec!['Q', 'Y'],
    };
    assert_eq!(adapt_wpm(20, &ok, 0), 21);
    assert_eq!(adapt_wpm(20, &ok, 1), 20);
    assert_eq!(adapt_wpm(11, &bad, 0), MIN_WPM);
    assert_eq!(adapt_wpm(MAX_WPM, &ok, 0), MAX_WPM);

    let mut s = RunnerStats::default();
    s.record(&ok, 25);
    s.record(&bad, 30);
    s.record(&bad, 30);
    s.finish_session(3);
    assert_eq!((s.qsos, s.calls_ok, s.top_wpm, s.best_score), (3, 1, 25, 3));
    assert_eq!(s.worst_chars(1), vec![('Q', 2)]);
  }
}
