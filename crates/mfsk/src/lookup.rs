//! Varicode 反查：位流里攒出的码字 → 字符。

use crate::varicode::VARICODE;

/// 码字（高位在先的二进制整数）→ 字符；码表里没有就返回 `None`。
///
/// 反查表只建一次（`OnceLock`）：接收热路径上每个候选都要反查，不能每次线性扫 256 个码字。
#[must_use]
pub fn varidec(code: u32) -> Option<u16> {
  static MAP: std::sync::OnceLock<std::collections::HashMap<u32, u16>> = std::sync::OnceLock::new();
  let map = MAP.get_or_init(|| {
    VARICODE
      .iter()
      .enumerate()
      .map(|(i, s)| {
        let key = s
          .bytes()
          .fold(0u32, |acc, b| (acc << 1) | u32::from(b == b'1'));
        (key, i as u16)
      })
      .collect()
  });
  map.get(&code).copied()
}

/// 字符 → 码字（`'1'` / `'0'` 串）。
#[must_use]
pub fn varienc(c: u8) -> &'static str {
  VARICODE[usize::from(c)]
}

/// 字符序列 → 展示文本：NUL / STX / EOT 等控制符去掉，`\r` 换成换行（MFSK 与 THOR 共用）。
pub fn render(chars: &[u16]) -> String {
  let mut out = String::new();
  for &c in chars {
    match c {
      13 => out.push('\n'),
      9 | 10 => out.push(char::from(c as u8)),
      0..=31 | 127 => {}
      _ => out.push(char::from(c as u8)),
    }
  }
  out
}

/// 文本可读性：可打印 ASCII（含空格与换行）加分，其余扣双倍。用来在多个候选译码之间选一个。
#[must_use]
pub fn readability(text: &str) -> i64 {
  text
    .chars()
    .map(|c| {
      if c.is_ascii_graphic() || c == ' ' || c == '\n' {
        1
      } else {
        -2
      }
    })
    .sum()
}

/// 文本里可打印字符（可打印 ASCII、空格、换行）的占比；空文本是 0。
///
/// 比 [`readability`] 的绝对分数更适合在**两次解同一段录音**之间选：乱码往往比正确的文本更长
/// （随机位流译出的字符更多），绝对分数会偏向乱码，占比不会。
#[must_use]
pub fn printable_fraction(text: &str) -> f64 {
  let total = text.chars().count();
  if total == 0 {
    return 0.0;
  }
  let ok = text
    .chars()
    .filter(|c| c.is_ascii_graphic() || *c == ' ' || *c == '\n')
    .count();
  ok as f64 / total as f64
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn codes_are_self_delimiting_and_round_trip() {
    // 字符边界靠 `001`：每个码字 1 开头、00 结尾、内部不含 `001`。
    for (i, c) in VARICODE.iter().enumerate() {
      assert!(c.starts_with('1') && c.ends_with("00"), "{i}: {c}");
      assert!(!c.contains("001"), "{i}: {c}");
      assert_eq!(varidec(u32::from_str_radix(c, 2).unwrap()), Some(i as u16));
    }
    assert_eq!(varidec(0), None);
    assert_eq!(varienc(b' '), "100");
  }

  #[test]
  fn readability_prefers_long_clean_text_but_fraction_does_not_care_about_length() {
    assert_eq!(readability("CQ\n"), 3);
    assert_eq!(readability("a\u{ad}"), -1);
    // 乱码更长时绝对分数更高，占比才是两次解码之间的公平比较。
    let clean = "CQ DE BG1AAA";
    let junk = "xx\u{8b}\u{94}\u{ad}yyzzwwqqrrttuuvvhhjjkk";
    assert!(readability(junk) > readability(clean));
    assert!(printable_fraction(junk) < printable_fraction(clean));
    assert_eq!(printable_fraction(clean), 1.0);
    assert_eq!(printable_fraction(""), 0.0);
  }
}
