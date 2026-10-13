//! 与独立参考实现的**双向互操作对拍**（进程内直连，无外部依赖）。
//!
//! 参考实现（`tests/reference/rtty_ref.rs`）按 ITA2 标准独立书写，与 `src/` 不共享
//! 任何代码：码表组织、换挡状态机、同步方式、音频合成方式都不同。双向验证：
//! **参考编码 → 本仓解码** 与 **本仓编码 → 参考解码**。

#[path = "reference/rtty_ref.rs"]
mod rtty_ref;

use ham_web_rtty::{BAUD, SHIFT_HZ, encode_text};

/// 本仓位流 → 音频（与参考的 synthesize 各自独立成文；1.5 停止位 = 位流 1 位 +
/// 调制层补 0.5 符号传号）。
fn modulate(bits: &[bool], mark_hz: f64) -> Vec<f32> {
  let spb = (f64::from(rtty_ref::SAMPLE_RATE) / BAUD).round() as usize;
  let half = (spb as f64 / 2.0).round() as usize;
  let mut out = Vec::new();
  let mut phase = 0.0f64;
  for chunk in bits.chunks(7) {
    for &b in chunk {
      let f = if b { mark_hz } else { mark_hz + SHIFT_HZ };
      for _ in 0..spb {
        out.push(phase.sin() as f32);
        phase += 2.0 * std::f64::consts::PI * f / f64::from(rtty_ref::SAMPLE_RATE);
      }
    }
    for _ in 0..half {
      out.push(phase.sin() as f32);
      phase += 2.0 * std::f64::consts::PI * mark_hz / f64::from(rtty_ref::SAMPLE_RATE);
    }
  }
  out
}

#[test]
fn ref_encode_rust_decode() {
  let text = "CQ CQ DE BG1AAA 599";
  let audio = rtty_ref::synthesize(text);
  let decoded =
    ham_web_rtty::decode_audio(&audio, rtty_ref::SAMPLE_RATE, rtty_ref::MARK).expect("应解出文本");
  assert_eq!(decoded, text, "参考编码 → 本仓解码");
}

#[test]
fn rust_encode_ref_decode() {
  let text = "CQ CQ DE BG1AAA 599";
  let bits = encode_text(text);
  let audio = modulate(&bits, rtty_ref::MARK);
  let decoded = rtty_ref::decode(&audio);
  assert_eq!(decoded, text, "本仓编码 → 参考解码");
}

#[test]
fn both_tables_agree_on_figures() {
  // 数字换挡往返：两张独立码表在 FIGS/LTRS 与数字符号上必须一致。
  let text = "CQ 25 W";
  let audio = rtty_ref::synthesize(text);
  let decoded =
    ham_web_rtty::decode_audio(&audio, rtty_ref::SAMPLE_RATE, rtty_ref::MARK).expect("应解出文本");
  assert_eq!(decoded, text, "含数字换挡的参考编码 → 本仓解码");
}
