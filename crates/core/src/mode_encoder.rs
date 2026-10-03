//! 数字模式报文编码：文本 → 摩尔斯（CW）与 RTTY（ITA2 / Baudot）比特流。

use crate::morse::code_of;

/// 字母移态码（LTRS）。
const LTRS: u8 = 31;
/// 数字移态码（FIGS）。
const FIGS: u8 = 27;

/// 文本 → 摩尔斯电码（点划序列，字符间空格、单词间 `/` 分隔）。
#[must_use]
pub fn text_to_morse(text: &str) -> String {
  let mut out = String::new();
  for ch in text.chars() {
    if ch == ' ' {
      if !out.is_empty() {
        out.push_str(" /");
      }
      continue;
    }
    if let Some(code) = code_of(ch) {
      if !out.is_empty() {
        out.push(' ');
      }
      out.push_str(code);
    }
  }
  out
}

/// ITA2 字母态编码值（5 位）。
fn letter_code(ch: char) -> Option<u8> {
  Some(match ch.to_ascii_uppercase() {
    'A' => 0b00011,
    'B' => 0b11001,
    'C' => 0b01110,
    'D' => 0b01001,
    'E' => 0b00001,
    'F' => 0b01101,
    'G' => 0b11010,
    'H' => 0b10100,
    'I' => 0b00110,
    'J' => 0b01011,
    'K' => 0b01111,
    'L' => 0b10010,
    'M' => 0b11100,
    'N' => 0b01100,
    'O' => 0b11000,
    'P' => 0b10110,
    'Q' => 0b10111,
    'R' => 0b01010,
    'S' => 0b00101,
    'T' => 0b10000,
    'U' => 0b00111,
    'V' => 0b11110,
    'W' => 0b10011,
    'X' => 0b11101,
    'Y' => 0b10101,
    'Z' => 0b10001,
    ' ' => 0b00100,
    '\r' => 0b00010,
    '\n' => 0b01000,
    _ => return None,
  })
}

/// ITA2 数字态编码值（5 位）。
fn figure_code(ch: char) -> Option<u8> {
  Some(match ch {
    '0' => 0b10110,
    '1' => 0b10111,
    '2' => 0b10011,
    '3' => 0b00001,
    '4' => 0b01010,
    '5' => 0b10000,
    '6' => 0b10101,
    '7' => 0b00111,
    '8' => 0b00110,
    '9' => 0b11000,
    _ => return None,
  })
}

/// 文本 → RTTY（ITA2）比特流，字母/数字切换时自动插入 LTRS/FIGS 移态码。
///
/// 返回空格分隔的 5 位二进制串，`11111` 为 LTRS、`11011` 为 FIGS。
#[must_use]
pub fn text_to_baudot_bits(text: &str) -> String {
  let mut out: Vec<u8> = Vec::new();
  let mut in_figs = false;
  for ch in text.chars() {
    if let Some(c) = letter_code(ch) {
      if in_figs {
        out.push(LTRS);
        in_figs = false;
      }
      out.push(c);
    } else if let Some(c) = figure_code(ch) {
      if !in_figs {
        out.push(FIGS);
        in_figs = true;
      }
      out.push(c);
    }
  }
  out
    .iter()
    .map(|c| format!("{c:05b}"))
    .collect::<Vec<_>>()
    .join(" ")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn morse_uses_code_of() {
    assert_eq!(text_to_morse("SOS"), "... --- ...");
    assert_eq!(text_to_morse("A B"), ".- / -...");
  }

  #[test]
  fn baudot_letter_no_shift() {
    assert_eq!(text_to_baudot_bits("A"), "00011");
    assert_eq!(text_to_baudot_bits("SOS"), "00101 11000 00101");
  }

  #[test]
  fn baudot_inserts_figs_shift_for_digits() {
    // A 后接 1：A=00011，切数字态 11011，1（Q 位）=10111。
    assert_eq!(text_to_baudot_bits("A1"), "00011 11011 10111");
  }

  #[test]
  fn baudot_returns_to_letters_shift() {
    // 1 后接 A：FIGS 10111，A=00011 前需回 LTRS 11111。
    assert_eq!(text_to_baudot_bits("1A"), "11011 10111 11111 00011");
  }
}
