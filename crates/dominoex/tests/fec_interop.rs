//! FEC 模式与独立参考实现（`tests/reference/dominoex_fec_ref.rs`，fldigi 逐行直译）的对拍。
//!
//! 三层：nibble 流**逐字节相同**（编码器）、双向互解（编码 → 对方解码）、
//! 音频端到端（含打坏 nibble 后由卷积码纠回）。

#[path = "reference/dominoex_fec_ref.rs"]
mod dominoex_fec_ref;

use dominoex_fec_ref::{Rx, Tx};
use ham_web_dominoex::fec::{FecDecoder, FecEncoder};
use ham_web_dominoex::{decode_audio, decode_audio_fec, encode_audio_fec, parse_mode};

const TEXT: &str = "CQ CQ DE BG1AAA BG1AAA PSE K 73";

fn ours(text: &str) -> Vec<u8> {
  let mut enc = FecEncoder::new();
  enc.clear_bits();
  let mut out = Vec::new();
  for _ in 0..4 {
    enc.encode_char(b' ', true, &mut out);
  }
  for b in text.bytes() {
    enc.encode_char(b, false, &mut out);
  }
  for _ in 0..10 {
    enc.encode_char(b' ', true, &mut out);
  }
  out
}

fn reference(text: &str) -> Vec<u8> {
  let mut tx = Tx::new();
  tx.clearbits();
  for _ in 0..4 {
    tx.send(b' ', true);
  }
  for b in text.bytes() {
    tx.send(b, false);
  }
  for _ in 0..10 {
    tx.send(b' ', true);
  }
  tx.symbols
}

fn ours_decode(ns: &[u8]) -> String {
  let mut dec = FecDecoder::new();
  let mut chars = Vec::new();
  for &n in ns {
    dec.feed(n, &mut chars);
  }
  chars
    .iter()
    .filter(|c| **c < 0x100)
    .map(|&c| char::from(c as u8))
    .collect()
}

#[test]
fn encoders_emit_identical_nibbles() {
  // 带小写、标点与 fldigi 会改写的字符（LF 不发、控制符换成 `_`）。
  let text = "Hello, World! 599\n\x07 <BK>";
  assert_eq!(ours(text), reference(text));
}

#[test]
fn reference_encode_ours_decode_and_back() {
  assert_eq!(ours_decode(&reference(TEXT)), TEXT);
  let mut rx = Rx::new();
  for n in ours(TEXT) {
    rx.feed(i32::from(n));
  }
  assert_eq!(rx.primary(), TEXT);
  // 二次信道的填充空格两边都识别成二次信道（不混进正文）。
  assert!(rx.text.iter().any(|&c| c == 0x100 | i32::from(b' ')));
}

#[test]
fn decoders_agree_on_a_damaged_stream() {
  // 打坏一部分 nibble：两个译码器不仅都要纠回原文，逐字符输出也必须相同。
  // 从引导段（4 个二次信道空格 ≈ 22 个 nibble）之后开始打：Viterbi 还没收敛时打坏，
  // 两边会一致地多译出一两个垃圾字符 —— 那是起始瞬态，不是纠错能力的问题。
  let mut ns = reference(TEXT);
  for i in (30..ns.len()).step_by(9) {
    ns[i] ^= 0b0010;
  }
  let mut rx = Rx::new();
  for &n in &ns {
    rx.feed(i32::from(n));
  }
  assert_eq!(ours_decode(&ns), rx.primary());
  assert_eq!(rx.primary(), TEXT);
}

#[test]
fn audio_round_trip_ex8_and_ex16() {
  for mode in ["8", "16"] {
    let (baud, spacing) = parse_mode(mode).unwrap();
    let audio = encode_audio_fec(TEXT, baud, spacing, 8000, 0.5).unwrap();
    let (primary, secondary) = decode_audio_fec(&audio, 8000, baud, spacing).expect("应解出文本");
    assert!(primary.contains(TEXT), "EX{mode} FEC：{primary:?}");
    assert!(secondary.trim().is_empty(), "填充只有空格：{secondary:?}");
    // 收发两端模式必须一致：FEC 录音按非 FEC 解，得不到原文。
    let plain = decode_audio(&audio, 8000, baud, spacing)
      .map(|(p, _)| p)
      .unwrap_or_default();
    assert!(!plain.contains(TEXT), "非 FEC 解码不该还原 FEC 录音");
  }
}
