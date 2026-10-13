//! CCIR 476 七位码：NAVTEX / SITOR-B / AMTOR 的字符集。
//!
//! 每个合法码恰含 4 个 mark 位（7 选 4 = 35 = 32 个 ITA2 字符 + 3 个控制码），
//! 因此「权重 ≠ 4」即检出错误 —— 这是 FEC 广播（NAVTEX）的字符级检错；
//! 纠错靠相隔五个字符的 DX / RX 副本配对（见 `message`）。
//!
//! 码表定义于 ITU-R M.476 / M.625。本表转写自开源实现 AI5GW/CCIR476
//! （GPL-3.0，baltic-lab.com），并经「常量 4-mark」性质逐码自验（见测试）。

/// 空闲 / 相位信号 1。
pub const ALPHA: u8 = 0x0F;
/// 相位信号 2。
pub const BETA: u8 = 0x33;
/// 重复/相位码：SITOR-B 与 ALPHA 交替定相，也用于 ARQ。
pub const REP: u8 = 0x66;
/// 字母换挡。
pub const LTRS: u8 = 0x5A;
/// 数字符号换挡。
pub const FIGS: u8 = 0x36;
/// 回车。
pub const CR: u8 = 0x78;
/// 换行。
pub const LF: u8 = 0x6C;
/// 空格。
pub const SP: u8 = 0x5C;
/// 空白（ITA2 NUL 在 CCIR 476 中的对应码）。
pub const BLK: u8 = 0x6A;

/// 字母表（下标 = 码值，LSB 先发）。
pub(crate) const LETTERS: [char; 128] = build_letters();
/// 数字符号表。
pub(crate) const FIGURES: [char; 128] = build_figures();

const fn build_letters() -> [char; 128] {
  let mut t = ['\0'; 128];
  t[0x47] = 'A';
  t[0x72] = 'B';
  t[0x1D] = 'C';
  t[0x53] = 'D';
  t[0x56] = 'E';
  t[0x1B] = 'F';
  t[0x35] = 'G';
  t[0x69] = 'H';
  t[0x4D] = 'I';
  t[0x17] = 'J';
  t[0x1E] = 'K';
  t[0x65] = 'L';
  t[0x39] = 'M';
  t[0x59] = 'N';
  t[0x71] = 'O';
  t[0x2D] = 'P';
  t[0x2E] = 'Q';
  t[0x55] = 'R';
  t[0x4B] = 'S';
  t[0x74] = 'T';
  t[0x4E] = 'U';
  t[0x3C] = 'V';
  t[0x27] = 'W';
  t[0x3A] = 'X';
  t[0x2B] = 'Y';
  t[0x63] = 'Z';
  t[0x5C] = ' ';
  t[0x78] = '\r';
  t[0x6C] = '\n';
  t
}

const fn build_figures() -> [char; 128] {
  let mut t = ['\0'; 128];
  t[0x2D] = '0';
  t[0x2E] = '1';
  t[0x27] = '2';
  t[0x56] = '3';
  t[0x55] = '4';
  t[0x74] = '5';
  t[0x2B] = '6';
  t[0x4E] = '7';
  t[0x4D] = '8';
  t[0x71] = '9';
  t[0x17] = '\'';
  t[0x1B] = '!';
  t[0x1D] = ':';
  t[0x1E] = '(';
  t[0x35] = '&';
  t[0x39] = '.';
  t[0x3A] = '/';
  t[0x3C] = '=';
  t[0x47] = '-';
  t[0x53] = '$';
  t[0x59] = ',';
  t[0x63] = '+';
  t[0x65] = ')';
  t[0x69] = '#';
  t[0x72] = '?';
  t[0x5C] = ' ';
  t[0x78] = '\r';
  t[0x6C] = '\n';
  t
}

/// 码是否合法（恰含 4 个 mark 位，且落在 7 位码空间内）。
///
/// `decode_code` 用码值直接索引 128 项的字符表：高位字节（≥ 0x80）里权重为 4
/// 的值若不排除，索引会越界 panic（公共 API 可被任意字节流触发）。
#[must_use]
pub fn is_valid(code: u8) -> bool {
  code < 128 && code.count_ones() == 4
}

/// 码权重不合法（≠ 4 个 mark 位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCode;

/// 按换挡状态解码一个码：`Ok(Some(char))`、`Ok(None)`（换挡 / 相位 / 重复码）或
/// `Err(InvalidCode)`（权重不合法）。
pub fn decode_code(code: u8, figs: bool) -> Result<Option<char>, InvalidCode> {
  if !is_valid(code) {
    return Err(InvalidCode);
  }
  if matches!(code, LTRS | FIGS | ALPHA | BETA | REP | BLK) {
    return Ok(None);
  }
  let ch = if figs { FIGURES } else { LETTERS }[code as usize];
  Ok((ch != '\0').then_some(ch))
}

/// 字符 → `(码, 是否需要数字换挡)`；不在表里的字符返回 `None`。
#[must_use]
pub fn encode_char(ch: char) -> Option<(u8, bool)> {
  let ch = ch.to_ascii_uppercase();
  if let Some(i) = LETTERS.iter().position(|&c| c == ch) {
    return Some((i as u8, false));
  }
  if let Some(i) = FIGURES.iter().position(|&c| c == ch) {
    return Some((i as u8, true));
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn high_bit_codes_are_rejected() {
    // 0xF0（4 个 mark 位）落在 128 项字符表之外：is_valid 必须拒绝，
    // decode_code 不能越界 panic。
    assert!(!is_valid(0xF0));
    assert!(!is_valid(0xE1));
    assert_eq!(decode_code(0xF0, false), Err(InvalidCode));
    assert_eq!(decode_code(0xFF, true), Err(InvalidCode));
  }

  #[test]
  fn every_defined_code_has_weight_four() {
    // 35 个合法码 = 全部 7 选 4 组合；控制码 + 字母 + 数字都要权重 4。
    let all: Vec<u8> = (0..=127u8).filter(|&c| is_valid(c)).collect();
    assert_eq!(all.len(), 35, "7 选 4 恰好 35 个码");
    for code in all {
      let l = LETTERS[code as usize];
      let f = FIGURES[code as usize];
      assert!(
        l != '\0' || f != '\0' || matches!(code, LTRS | FIGS | ALPHA | BETA | REP | BLK),
        "0x{code:02X} 权重 4 但在两张表里都没有定义"
      );
    }
  }

  #[test]
  fn round_trips_letters_digits_and_punctuation() {
    for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789 -?/:().,'=+$#!&".chars() {
      let (code, want_figs) = encode_char(ch).unwrap_or_else(|| panic!("未收录 {ch}"));
      let figs = want_figs;
      let Some(got) = decode_code(code, figs).expect("合法") else {
        panic!("{ch} 不该是控制码");
      };
      assert_eq!(got, ch.to_ascii_uppercase(), "{ch}");
    }
  }

  #[test]
  fn shift_codes_are_state_changes() {
    assert_eq!(decode_code(LTRS, true), Ok(None));
    assert_eq!(decode_code(FIGS, false), Ok(None));
    assert_eq!(decode_code(ALPHA, false), Ok(None));
    assert_eq!(decode_code(BETA, false), Ok(None));
    assert_eq!(decode_code(REP, false), Ok(None));
    assert_eq!(decode_code(BLK, false), Ok(None));
  }

  #[test]
  fn invalid_weight_is_rejected() {
    assert_eq!(decode_code(0x00, false), Err(InvalidCode)); // 权重 0
    assert_eq!(decode_code(0x77, false), Err(InvalidCode)); // 权重 6
    assert_eq!(decode_code(0x03, false), Err(InvalidCode)); // 权重 2
  }
}
