//! 国际摩尔斯电码（ITU）对照表：字母、数字、常用标点与信号时值标准。
//!
//! 点划序列用 `.` 表示点、`-` 表示划；字母对应的语音字母（字母解释法）统一由
//! [`crate::phonetic`] 维护，避免两处重复。

use crate::koch::Timing;

/// 一个摩尔斯电码字符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorseChar {
  /// 字符本身，如 `A`、`0`、`,`。
  pub ch: &'static str,
  /// 点划序列：`.` 表示点、`-` 表示划。
  pub code: &'static str,
}

const fn mc(ch: &'static str, code: &'static str) -> MorseChar {
  MorseChar { ch, code }
}

/// 字母表 A–Z（语音字母见 [`crate::phonetic::PHONETIC`]）。
pub const LETTERS: &[MorseChar] = &[
  mc("A", ".-"),
  mc("B", "-..."),
  mc("C", "-.-."),
  mc("D", "-.."),
  mc("E", "."),
  mc("F", "..-."),
  mc("G", "--."),
  mc("H", "...."),
  mc("I", ".."),
  mc("J", ".---"),
  mc("K", "-.-"),
  mc("L", ".-.."),
  mc("M", "--"),
  mc("N", "-."),
  mc("O", "---"),
  mc("P", ".--."),
  mc("Q", "--.-"),
  mc("R", ".-."),
  mc("S", "..."),
  mc("T", "-"),
  mc("U", "..-"),
  mc("V", "...-"),
  mc("W", ".--"),
  mc("X", "-..-"),
  mc("Y", "-.--"),
  mc("Z", "--.."),
];

/// 数字 0–9。
pub const DIGITS: &[MorseChar] = &[
  mc("0", "-----"),
  mc("1", ".----"),
  mc("2", "..---"),
  mc("3", "...--"),
  mc("4", "....-"),
  mc("5", "....."),
  mc("6", "-...."),
  mc("7", "--..."),
  mc("8", "---.."),
  mc("9", "----."),
];

/// 常用标点符号。
pub const PUNCTUATION: &[MorseChar] = &[
  mc(".", ".-.-.-"),
  mc(",", "--..--"),
  mc("?", "..--.."),
  mc("'", ".----."),
  mc("!", "-.-.--"),
  mc("/", "-..-."),
  mc("(", "-.--."),
  mc(")", "-.--.-"),
  mc("&", ".-..."),
  mc(":", "---..."),
  mc(";", "-.-.-."),
  mc("=", "-...-"),
  mc("+", ".-.-."),
  mc("-", "-....-"),
  mc("_", "..--.-"),
  mc("\"", ".-..-."),
  mc("$", "...-..-"),
  mc("@", ".--.-."),
];

/// 信号时值标准（以一个「点」时间为基准）。
pub const TIMING: &[(&str, &str)] = &[
  ("点", "1"),
  ("划", "3"),
  ("字符内点划间隔", "1"),
  ("字符间隔", "3"),
  ("单词（组）间隔", "7"),
];

/// CW 通联常用单词/缩语（用于整句听抄练习）。
pub const COMMON_WORDS: &[&str] = &[
  "CQ", "DE", "RST", "QTH", "NAME", "RIG", "ANT", "PWR", "WX", "QSL", "QSO", "DX", "PSE", "UR",
  "MY", "OK", "TNX", "R", "S", "T", "K", "SK", "GL", "GM", "GE", "HI", "OM", "OP", "73",
];

/// 按字符查找摩尔斯点划序列（`.`` 表示点、`-` 表示划）。
#[must_use]
pub fn code_of(ch: char) -> Option<&'static str> {
  let u = ch.to_ascii_uppercase();
  LETTERS
    .iter()
    .chain(DIGITS)
    .chain(PUNCTUATION)
    .find(|m| m.ch.starts_with(u))
    .map(|m| m.code)
}

/// 把点划序列解析为「字符 → (点划, 是否单词结尾)」。
fn tokenize(code: &str) -> Vec<(Vec<char>, bool)> {
  let mut chars: Vec<(Vec<char>, bool)> = Vec::new();
  for token in code.split(' ').filter(|s| !s.is_empty()) {
    if token == "/" {
      if let Some(last) = chars.last_mut() {
        last.1 = true;
      }
    } else {
      chars.push((
        token.chars().filter(|&c| c == '.' || c == '-').collect(),
        false,
      ));
    }
  }
  chars
}

/// 逐符号时间区间（秒）：返回每个点/划的 `(开始, 结束)`，顺序对应 `code` 中的点划符号，
/// 忽略空格与 `/`。起始偏移与播放调度的 0.02s 前置延迟保持一致。
#[must_use]
pub fn morse_symbol_times(code: &str, timing: Timing) -> Vec<(f64, f64)> {
  let chars = tokenize(code);
  let dot = timing.dot;
  let mut out = Vec::new();
  let mut t = 0.02;
  for (ci, (marks, word_end)) in chars.iter().enumerate() {
    for (mi, &c) in marks.iter().enumerate() {
      let dur = if c == '.' { dot } else { 3.0 * dot };
      out.push((t, t + dur));
      t += dur;
      if mi + 1 < marks.len() {
        t += dot;
      }
    }
    if ci + 1 < chars.len() {
      t += if *word_end {
        timing.word_gap
      } else {
        timing.char_gap
      };
    }
  }
  out
}

/// 逐字符时间区间（秒）：返回每个字符（按空格分组）的 `(开始, 结束)`。
#[must_use]
pub fn morse_char_times(code: &str, timing: Timing) -> Vec<(f64, f64)> {
  let chars = tokenize(code);
  let dot = timing.dot;
  let mut out = Vec::new();
  let mut t = 0.02;
  for (ci, (marks, word_end)) in chars.iter().enumerate() {
    let start = t;
    for (mi, &c) in marks.iter().enumerate() {
      let dur = if c == '.' { dot } else { 3.0 * dot };
      t += dur;
      if mi + 1 < marks.len() {
        t += dot;
      }
    }
    out.push((start, t));
    if ci + 1 < chars.len() {
      t += if *word_end {
        timing.word_gap
      } else {
        timing.char_gap
      };
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_points_are_valid() {
    for c in LETTERS.iter().chain(DIGITS).chain(PUNCTUATION) {
      assert!(!c.ch.is_empty(), "字符不能为空：{c:?}");
      assert!(!c.code.is_empty(), "点划序列不能为空：{c:?}");
      assert!(
        c.code.chars().all(|x| x == '.' || x == '-'),
        "点划序列只能含 . 和 -：{c:?}"
      );
    }
  }

  #[test]
  fn alphabet_is_complete() {
    assert_eq!(LETTERS.len(), 26);
    assert_eq!(DIGITS.len(), 10);
  }

  #[test]
  fn spot_checks() {
    assert_eq!(LETTERS[0].code, ".-");
    assert_eq!(DIGITS[9].code, "----.");
    assert_eq!(PUNCTUATION[0].code, ".-.-.-");
    assert_eq!(
      LETTERS.iter().find(|c| c.ch == "S").map(|c| c.code),
      Some("...")
    );
  }

  #[test]
  fn symbol_and_char_times_align_with_standard_timing() {
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
    // 20/20 WPM：点 0.06s、字符间隔 0.18s、单词间隔 0.42s
    let timing = crate::koch::farnsworth(20.0, 20.0);
    let sym = morse_symbol_times(".- -...", timing);
    assert_eq!(sym.len(), 6);
    assert!(near(sym[0], (0.02, 0.08)));
    assert!(near(sym[1], (0.14, 0.32)));
    assert!(near(sym[2], (0.50, 0.68)));
    assert!(near(sym[5], (0.98, 1.04)));

    let chars = morse_char_times(".- -...", timing);
    assert_eq!(chars.len(), 2);
    assert!(near(chars[0], (0.02, 0.32)));
    assert!(near(chars[1], (0.50, 1.04)));
  }
}
