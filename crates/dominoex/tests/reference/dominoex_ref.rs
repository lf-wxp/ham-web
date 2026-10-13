//! DominoEX **参考实现**（互操作对拍用，独立于 `src/`）。
//!
//! 转写自 fldigi（GPL-3.0，https://github.com/w1hkj/fldigi ，`src/dominoex/`）
//! —— 原为纯 Python 移植，现改为 Rust 版本（与 crate 核心**不共享代码**，
//! 只共享上游出处与生成的 Varicode 表数据 `ham_web_dominoex::varicode`）。
//!
//! 算法：18 音调 IFK+（`tone = (prev + 2 + nibble) mod 18`，差分携带数据）+
//! Nibble Varicode（最新 nibble 在最低 4 位打包）。

use std::f64::consts::PI;

use ham_web_dominoex::varicode::{VARICODE, VARIDECODE};

/// 音调数。
pub const NUMTONES: usize = 18;
/// 音调网格的中心频率。
pub const BASEFREQ: f64 = 1000.0;

/// 子模式参数：(波特率, 音调间隔, 采样率)。
#[must_use]
pub fn mode(which: &str) -> Option<(f64, f64, u32)> {
  match which {
    "4" => Some((3.90625, 7.8125, 8000)),
    "5" => Some((5.3833, 10.766, 11025)),
    "8" => Some((7.8125, 15.625, 8000)),
    "11" => Some((10.766, 10.766, 11025)),
    "16" => Some((15.625, 15.625, 8000)),
    "22" => Some((21.533, 21.533, 11025)),
    _ => None,
  }
}

/// 字符 → nibble 序列（首码 + MSB=1 的续码）。
#[must_use]
pub fn nibbles_of(c: usize) -> Vec<u8> {
  let code = VARICODE[c];
  let mut out = vec![code[0]];
  for &n in &code[1..] {
    if n & 0x8 != 0 {
      out.push(n);
    } else {
      break;
    }
  }
  out
}

/// 文本 → 音频样本（前导/尾随空闲符 + IFK+ 合成，连续相位）。
#[must_use]
pub fn synth(text: &str, baud: f64, spacing: f64, rate: u32, idle: f64) -> Vec<f32> {
  let symlen = (f64::from(rate) / baud).round() as usize;
  let idle_chars = (idle * baud / 3.0).round() as usize;
  let mut nibs = Vec::new();
  for _ in 0..idle_chars {
    nibs.extend(nibbles_of(0));
  }
  for b in text.bytes() {
    nibs.extend(nibbles_of(usize::from(b)));
  }
  for _ in 0..idle_chars.max(4) {
    nibs.extend(nibbles_of(0));
  }

  let mut out: Vec<f32> = Vec::new();
  let mut prev = 0usize;
  for n in nibs {
    prev = (prev + 2 + usize::from(n)) % NUMTONES;
    let f = BASEFREQ - NUMTONES as f64 * spacing / 2.0 + (prev as f64 + 0.5) * spacing;
    for _ in 0..symlen {
      out.push((2.0 * PI * f * out.len() as f64 / f64::from(rate)).sin() as f32);
    }
  }
  out
}

/// 单频 Goertzel 功率。
fn goertzel(xs: &[f32], freq: f64, rate: u32) -> f64 {
  let w = 2.0 * PI * freq / f64::from(rate);
  let coeff = 2.0 * w.cos();
  let mut s0 = 0.0f64;
  let mut s1 = 0.0f64;
  for &x in xs {
    let s = f64::from(x) + coeff * s1 - s0;
    s0 = s1;
    s1 = s;
  }
  s1 * s1 + s0 * s0 - coeff * s1 * s0
}

/// 音频样本 → 文本（¼ 音调间隔网格上测最强频率 → 差分圆整 −2 mod 18 → nibble）。
#[must_use]
pub fn decode(samples: &[f32], rate: u32, baud: f64, spacing: f64) -> String {
  let symlen = (f64::from(rate) / baud).round() as usize;
  let band = NUMTONES as f64 * spacing;
  let lo = BASEFREQ - band / 2.0;
  let hi = BASEFREQ + band / 2.0;
  let step = spacing / 4.0;

  let peak = |win: &[f32]| -> f64 {
    let mut best = (lo, -1.0f64);
    let mut f = lo;
    while f <= hi {
      let p = goertzel(win, f, rate);
      if p > best.1 {
        best = (f, p);
      }
      f += step;
    }
    best.0
  };

  // 符号对齐：前 0.5 秒内峰值能量和最大的偏移（4 样本步进）。
  let scan = (0.5 * f64::from(rate)) as usize;
  let mut best_off = (0usize, -1.0f64);
  let mut off = 0usize;
  while off < scan.min(symlen) {
    let mut total = 0.0f64;
    let mut i = off;
    let mut cnt = 0;
    while i + symlen <= samples.len() && cnt < 32 {
      let f = peak(&samples[i..i + symlen]);
      total += goertzel(&samples[i..i + symlen], f, rate);
      i += symlen;
      cnt += 1;
    }
    if total > best_off.1 {
      best_off = (off, total);
    }
    off += 4;
  }

  let mut freqs = Vec::new();
  let mut i = best_off.0;
  while i + symlen <= samples.len() {
    freqs.push(peak(&samples[i..i + symlen]));
    i += symlen;
  }
  let mut nibs = Vec::new();
  for w in freqs.windows(2) {
    let d = (w[1] - w[0]) / spacing;
    nibs.push(((d.round() as i64 - 2).rem_euclid(NUMTONES as i64)) as u8);
  }
  if !nibs.is_empty() {
    nibs.remove(0); // 首个差值无有效参照
  }

  // Nibble Varicode 装配：MSB=0 的 nibble 开启新字符，最新 nibble 在最低 4 位。
  let mut buf = [0u8; 3];
  let mut count = 0usize;
  let mut text = String::new();
  // 结算已攒下的 nibble（与 crate 侧同一套边界：count 0 或查表越界都只是「没有字符」）。
  let flush = |buf: &[u8; 3], count: usize, text: &mut String| {
    if count == 0 {
      return;
    }
    let mut packed = 0usize;
    for i2 in 0..count {
      packed |= usize::from(buf[count - 1 - i2]) << (4 * i2);
    }
    if let Some(&v) = VARIDECODE.get(packed)
      && v > 0
    {
      text.push(char::from((if v < 256 { v } else { v - 256 }) as u8));
    }
  };
  for nb in nibs {
    // 16/17 = 相邻窗口同音调（静音 / 噪声），不是字符数据：结算后丢弃。
    let invalid = nb > 0x0F;
    if invalid || nb & 0x8 == 0 {
      flush(&buf, count, &mut text);
      count = 0;
    }
    if !invalid && count < 3 {
      buf[count] = nb;
      count += 1;
    }
  }
  text
}
