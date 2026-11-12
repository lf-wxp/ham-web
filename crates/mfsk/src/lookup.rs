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
}
