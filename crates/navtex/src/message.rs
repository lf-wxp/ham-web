//! SITOR-B 字符级 FEC → NAVTEX 完整报文。
//!
//! 每个字符发送两次，DX 在 RX 前 35 位（五个七位字符），两条通道交替。
//! 位序与延迟参考 fldigi `src/navtex/navtex.cxx` 的 `bytes_to_code` / `fec_offset`；
//! 本实现只做硬判决，不宣称覆盖真实录音中的时钟漂移和软判决接收。

use crate::codec::{ALPHA, BETA, BLK, FIGS, LTRS, REP, decode_code, is_valid};

/// 一条解出的完整报文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavtexMessage {
  pub transmitter: char,
  pub category: char,
  pub serial: u8,
  pub text: String,
  pub corrected: usize,
  pub errors: usize,
  pub chars: usize,
  /// 只对完整报文计算未纠正字符错误率，成功纠正不算残余错误。
  pub accepted: bool,
}

/// 去交织后的字符及纠错状态。
#[derive(Clone, Copy)]
struct Symbol {
  code: Option<u8>,
  corrected: bool,
  slot: usize,
}

fn control(code: u8) -> bool {
  matches!(code, ALPHA | BETA | REP | BLK)
}

fn combine(dx: u8, rx: u8) -> Symbol {
  let a = is_valid(dx);
  let b = is_valid(rx);
  let code = match (a, b) {
    _ if control(dx) || control(rx) => Some(ALPHA),
    (true, true) if dx == rx => Some(rx),
    (true, false) => Some(dx),
    (false, true) => Some(rx),
    _ => None,
  };
  Symbol {
    code,
    corrected: a != b,
    slot: 0,
  }
}

fn deinterleave(codes: &[u8], rx_phase: usize) -> Vec<Symbol> {
  let mut out = Vec::with_capacity(codes.len() / 2);
  for i in (5..codes.len()).filter(|i| i % 2 == rx_phase) {
    let dx = codes[i - 5];
    let rx = codes[i];
    let mut symbol = combine(dx, rx);
    symbol.slot = i;
    out.push(symbol);
  }
  out
}

fn parse_symbols(symbols: &[Symbol]) -> Vec<(usize, usize, NavtexMessage)> {
  let mut figs = false;
  let mut chars = Vec::new();
  for symbol in symbols {
    match symbol.code {
      Some(LTRS) => figs = false,
      Some(FIGS) => figs = true,
      Some(code) if control(code) => {
        chars.push(('\0', false, false, symbol.slot));
        figs = false;
      }
      Some(code) => {
        if let Ok(Some(ch)) = decode_code(code, figs) {
          chars.push((ch, symbol.corrected, false, symbol.slot));
        }
      }
      None => chars.push(('■', false, true, symbol.slot)),
    }
  }
  let mut out = Vec::new();
  let mut start = 0;
  while start + 9 <= chars.len() {
    if !chars[start..start + 5]
      .iter()
      .map(|c| c.0)
      .eq("ZCZC ".chars())
    {
      start += 1;
      continue;
    }
    let header = &chars[start + 5..start + 9];
    if !header[0].0.is_ascii_uppercase()
      || !header[1].0.is_ascii_uppercase()
      || !header[2].0.is_ascii_digit()
      || !header[3].0.is_ascii_digit()
    {
      start += 1;
      continue;
    }
    let mut body_start = start + 9;
    if chars.get(body_start).map(|c| c.0) != Some('\r')
      || chars.get(body_start + 1).map(|c| c.0) != Some('\n')
    {
      start += 1;
      continue;
    }
    while chars
      .get(body_start)
      .is_some_and(|c| c.0.is_ascii_whitespace())
    {
      body_start += 1;
    }
    let max_end = (body_start + 2048).min(chars.len());
    let mut end = body_start;
    let mut terminated = false;
    while end + 4 <= max_end {
      if chars[end].0 == '\0' {
        break;
      }
      if chars[end..end + 4].iter().map(|c| c.0).eq("NNNN".chars()) {
        terminated = true;
        break;
      }
      // 新报头出现意味着前一报文截断，不能吞到下一报文的结束标记。
      if end + 5 <= max_end && chars[end..end + 5].iter().map(|c| c.0).eq("ZCZC ".chars()) {
        break;
      }
      end += 1;
    }
    if !terminated {
      start += 1;
      continue;
    }
    let body = &chars[body_start..end];
    let text: String = body.iter().map(|c| c.0).collect();
    let text = text.trim_end().to_owned();
    if !text.is_empty() {
      let errors = body.iter().filter(|c| c.2).count();
      out.push((
        chars[start].3,
        chars[end + 3].3 + 1,
        NavtexMessage {
          transmitter: header[0].0,
          category: header[1].0,
          serial: (header[2].0 as u8 - b'0') * 10 + header[3].0 as u8 - b'0',
          text,
          corrected: body.iter().filter(|c| c.1).count(),
          errors,
          chars: body.len(),
          accepted: errors as f64 / (body.len().max(1) as f64) < 0.04,
        },
      ));
    }
    start = end + 4;
  }
  out
}

/// 诊断用：在已经去交织的位流中寻找连续 ZCZC（LSB 先发），不是原始 FEC 同步器。
#[must_use]
pub fn find_zczc(bits: &[bool], max_errors: usize) -> Vec<usize> {
  let pattern: Vec<bool> = [0x63u8, 0x1D, 0x63, 0x1D]
    .into_iter()
    .flat_map(|code| (0..7).map(move |bit| code & (1 << bit) != 0))
    .collect();
  bits
    .windows(28)
    .enumerate()
    .filter_map(|(i, window)| {
      (window.iter().zip(&pattern).filter(|(a, b)| a != b).count() <= max_errors).then_some(i)
    })
    .collect()
}

/// 原始 SITOR-B 位流 → 完整 NAVTEX 报文。尝试七个位网格、两个通道相位及两条极性。
/// 重叠时域候选按残余错误率择优；不重叠的报文保留，不按站台/序号合并。
#[must_use]
pub fn decode_messages(bits: &[bool]) -> Vec<NavtexMessage> {
  let mut candidates = Vec::new();
  for offset in 0..7.min(bits.len()) {
    let codes: Vec<u8> = bits[offset..]
      .as_chunks::<7>()
      .0
      .iter()
      .map(|chunk| {
        chunk
          .iter()
          .enumerate()
          .fold(0, |code, (i, bit)| code | (u8::from(*bit) << i))
      })
      .collect();
    for inverted in [false, true] {
      let codes = if inverted {
        codes.iter().map(|c| c ^ 0x7F).collect()
      } else {
        codes.clone()
      };
      for phase in 0..2 {
        for (start, end, message) in parse_symbols(&deinterleave(&codes, phase)) {
          candidates.push((offset + start * 7, offset + end * 7, message));
        }
      }
    }
  }
  candidates.sort_by(|a, b| {
    let quality = |m: &NavtexMessage| m.errors as f64 / m.chars.max(1) as f64;
    quality(&a.2)
      .total_cmp(&quality(&b.2))
      .then(a.2.corrected.cmp(&b.2.corrected))
      .then(b.2.chars.cmp(&a.2.chars))
      .then(a.0.cmp(&b.0))
  });
  let mut accepted: Vec<(usize, usize, NavtexMessage)> = Vec::new();
  for candidate in candidates {
    if !accepted
      .iter()
      .any(|other| candidate.0 < other.1 && other.0 < candidate.1)
    {
      accepted.push(candidate);
    }
  }
  accepted.sort_by_key(|m| m.0);
  accepted
    .into_iter()
    .map(|(_, _, message)| message)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  // 固定 CCIR-476 码，独立于生产生成器：ZCZC FA<FIGS>79<LTRS><CR><LF>HELLO<CR><LF>NNNN。
  fn codes() -> Vec<u8> {
    vec![
      0x63, 0x1D, 0x63, 0x1D, 0x5C, 0x1B, 0x47, 0x36, 0x4E, 0x71, 0x5A, 0x78, 0x6C, 0x69, 0x56,
      0x65, 0x65, 0x71, 0x78, 0x6C, 0x59, 0x59, 0x59, 0x59,
    ]
  }

  fn transmit(data: &[u8]) -> Vec<u8> {
    let mut stream = vec![REP, ALPHA, REP, ALPHA, REP, ALPHA];
    let base = stream.len();
    for i in 0..data.len() + 2 {
      stream.push(data.get(i).copied().unwrap_or(BLK));
      stream.push(
        i.checked_sub(2)
          .and_then(|j| data.get(j))
          .copied()
          .unwrap_or(ALPHA),
      );
    }
    assert_eq!(stream[base + 5], data[0]);
    stream
  }

  fn bits(codes: &[u8]) -> Vec<bool> {
    codes
      .iter()
      .flat_map(|code| (0..7).map(move |b| code & (1 << b) != 0))
      .collect()
  }

  #[test]
  fn standard_interleaved_header_and_body_are_decoded() {
    let raw = transmit(&codes());
    for leading in 0..14 {
      let mut input = vec![false; leading];
      input.extend(bits(&raw));
      let messages = decode_messages(&input);
      assert_eq!(messages.len(), 1, "偏移 {leading}");
      let m = &messages[0];
      assert_eq!((m.transmitter, m.category, m.serial), ('F', 'A', 79));
      assert_eq!(m.text, "HELLO");
      assert!(m.accepted);
    }
  }

  #[test]
  fn a_bad_rx_character_is_corrected_from_dx_five_characters_earlier() {
    let mut raw = transmit(&codes());
    raw[6 + 2 * 13 + 5] ^= 1;
    let messages = decode_messages(&bits(&raw));
    assert_eq!(messages[0].text, "HELLO");
    assert_eq!(messages[0].corrected, 1);
    assert_eq!(messages[0].errors, 0);
    assert!(messages[0].accepted);
  }

  #[test]
  fn conflicting_valid_copies_are_not_reported_as_corrected() {
    let mut raw = transmit(&codes());
    raw[6 + 2 * 13 + 5] = 0x47;
    let messages = decode_messages(&bits(&raw));
    assert_eq!(messages[0].errors, 1);
    assert!(!messages[0].accepted);
  }

  #[test]
  fn controls_and_missing_line_separators_cannot_make_a_complete_message() {
    let mut interrupted = codes();
    interrupted.insert(interrupted.len() - 2, ALPHA);
    assert!(decode_messages(&bits(&transmit(&interrupted))).is_empty());
    let mut interrupted_body = codes();
    interrupted_body.insert(15, REP);
    assert!(decode_messages(&bits(&transmit(&interrupted_body))).is_empty());
    let mut no_newline = codes();
    no_newline.splice(11..13, [0x5C]);
    assert!(decode_messages(&bits(&transmit(&no_newline))).is_empty());
  }

  #[test]
  fn rep_alpha_phasing_cannot_borrow_a_later_fragments_terminator() {
    assert_eq!(combine(REP, ALPHA).code, Some(ALPHA));
    let mut interrupted = codes();
    interrupted.truncate(interrupted.len() - 4);
    interrupted.splice(13..18, std::iter::repeat_n(0x69, 400));
    let mut raw = transmit(&interrupted);
    for _ in 0..4 {
      raw.extend([REP, ALPHA]);
    }
    raw.extend(transmit(&[LTRS, 0x59, 0x59, 0x59, 0x59]));
    assert!(decode_messages(&bits(&raw)).is_empty());
  }

  #[test]
  fn incomplete_or_uninterleaved_messages_are_rejected() {
    let mut truncated = codes();
    truncated.truncate(truncated.len() - 4);
    assert!(decode_messages(&bits(&transmit(&truncated))).is_empty());
    assert!(decode_messages(&bits(&codes())).is_empty());
    let mut bad_header = codes();
    bad_header[4] = 0x47;
    assert!(decode_messages(&bits(&transmit(&bad_header))).is_empty());
  }

  #[test]
  fn consecutive_same_number_messages_in_one_fec_stream_are_not_deduplicated() {
    let joined: Vec<u8> = codes().into_iter().chain(codes()).collect();
    let messages = decode_messages(&bits(&transmit(&joined)));
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().all(|m| m.text == "HELLO" && m.serial == 79));
  }

  #[test]
  fn repeated_message_numbers_survive_mid_stream_grid_and_polarity_changes() {
    let first = bits(&transmit(&codes()));
    for gap in [1, 7, 15] {
      let mut stream = first.clone();
      stream.extend(std::iter::repeat_n(false, gap));
      stream.extend(first.iter().map(|b| !b));
      let messages = decode_messages(&stream);
      assert_eq!(messages.len(), 2, "中段偏移 {gap}");
      assert!(messages.iter().all(|m| m.text == "HELLO" && m.serial == 79));
    }
  }

  #[test]
  fn message_limit_does_not_turn_truncation_into_acceptance() {
    let mut long = codes();
    long.splice(13..18, std::iter::repeat_n(0x69, 2050));
    assert!(decode_messages(&bits(&transmit(&long))).is_empty());
  }

  #[test]
  fn noise_yields_no_messages() {
    let mut seed = 42u32;
    let input: Vec<bool> = (0..3000)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed & 0x8000 != 0
      })
      .collect();
    assert!(decode_messages(&input).is_empty());
  }
}
