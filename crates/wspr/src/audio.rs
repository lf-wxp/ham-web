//! WSPR 音频合成与 FSK 解调。

use std::f64::consts::TAU;

use super::decode::{Message, decode_symbols};
use super::{SYMBOL_COUNT, SYMBOL_SECONDS, TONE_SPACING_HZ};

/// 由符号合成连续相位 4-FSK 音频样本。
///
/// `rate` 为采样率；每符号样本数 = `rate × SYMBOL_SECONDS`（12000 Hz 时恰为 8192）。
pub fn synthesize(symbols: &[u8; 162], base_hz: f64, rate: u32) -> Vec<f32> {
  let per_symbol = (rate as f64 * SYMBOL_SECONDS).round() as usize;
  let mut out = Vec::with_capacity(SYMBOL_COUNT * per_symbol);
  let mut phase = 0.0f64;
  for &s in symbols {
    let step = TAU * (base_hz + s as f64 * TONE_SPACING_HZ) / rate as f64;
    for _ in 0..per_symbol {
      out.push(phase.sin() as f32);
      phase = (phase + step) % TAU;
    }
  }
  out
}

/// Goertzel 算法：在 `freq` 处估计信号功率。
fn goertzel(x: &[f32], freq: f64, rate: f64) -> f64 {
  let k = (x.len() as f64 * freq / rate).round();
  let w = TAU * k / x.len() as f64;
  let coeff = 2.0 * w.cos();
  let (mut s1, mut s2) = (0.0f64, 0.0f64);
  for &v in x {
    let s0 = v as f64 + coeff * s1 - s2;
    s2 = s1;
    s1 = s0;
  }
  s1 * s1 + s2 * s2 - coeff * s1 * s2
}

/// 从音频样本解码消息（符号边界从 0 对齐，`base_hz` 为 4 音调基准频率）。
pub fn decode_audio(samples: &[f32], base_hz: f64, rate: u32) -> Option<Message> {
  let per_symbol = (rate as f64 * SYMBOL_SECONDS).round() as usize;
  if samples.len() < SYMBOL_COUNT * per_symbol {
    return None;
  }
  let mut symbols = [0u8; 162];
  for (i, sym) in symbols.iter_mut().enumerate() {
    let start = i * per_symbol;
    let seg = &samples[start..start + per_symbol];
    let mut best = 0u8;
    let mut best_power = f64::NEG_INFINITY;
    for t in 0..4u8 {
      let power = goertzel(seg, base_hz + t as f64 * TONE_SPACING_HZ, rate as f64);
      if power > best_power {
        best_power = power;
        best = t;
      }
    }
    *sym = best;
  }
  decode_symbols(&symbols)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::encode::encode;

  #[test]
  fn audio_roundtrip() {
    // 用 12000 Hz 采样率（每符号 8192 样本）加速测试。
    let rate = 12_000u32;
    let base = 1500.0;
    let symbols = encode("AA0NT", "EM18", 20).expect("应编码成功");
    let audio = synthesize(&symbols, base, rate);
    let msg = decode_audio(&audio, base, rate).expect("应解码成功");
    assert_eq!(msg.to_string(), "AA0NT EM18 20");
  }
}
