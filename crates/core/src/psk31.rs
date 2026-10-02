//! PSK31（BPSK）解码：Varicode 编码表与音频解调。
//!
//! PSK31 是 31.25 波特的差分 BPSK 模式，字符用 Varicode 变长编码（位 0 表示载波相位
//! 反转、位 1 表示稳定载波），字符间以至少两个连续 0 分隔。本模块提供从单声道音频
//! 样本解码文本的完整链路：重采样 → I/Q 下变频 → 低通 → 符号采样 → 差分判决 → Varicode。

/// 波特率。
const BAUD: f64 = 31.25;
/// 工作采样率（与波特率成整数比，每符号 256 采样）。
const WORK_RATE: u32 = 8000;
/// 每符号采样数 = 8000 / 31.25。
const SYMBOL_LEN: usize = 256;

/// 空编码占位（控制字符 / DEL）。
const EMPTY: &str = "";

/// PSK31 Varicode 编码表（索引 = ASCII 字节值，位串左位先；数据源 fldigi `varicodetab1`）。
#[rustfmt::skip]
pub const VARICODE: [&str; 128] = [
  EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY,
  EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY,
  EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY,
  EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY,
  "1",          // 32 space
  "111111111",  // 33 !
  "101011111",  // 34 "
  "111110101",  // 35 #
  "111011011",  // 36 $
  "1011010101", // 37 %
  "1010111011", // 38 &
  "101111111",  // 39 '
  "11111011",   // 40 (
  "11110111",   // 41 )
  "101101111",  // 42 *
  "111011111",  // 43 +
  "1110101",    // 44 ,
  "110101",     // 45 -
  "1010111",    // 46 .
  "110101111",  // 47 /
  "10110111",   // 48 0
  "10111101",   // 49 1
  "11101101",   // 50 2
  "11111111",   // 51 3
  "101110111",  // 52 4
  "101011011",  // 53 5
  "101101011",  // 54 6
  "110101101",  // 55 7
  "110101011",  // 56 8
  "110110111",  // 57 9
  "11110101",   // 58 :
  "110111101",  // 59 ;
  "111101101",  // 60 <
  "1010101",    // 61 =
  "111010111",  // 62 >
  "1010101111", // 63 ?
  "1010111101", // 64 @
  "1111101",    // 65 A
  "11101011",   // 66 B
  "10101101",   // 67 C
  "10110101",   // 68 D
  "1110111",    // 69 E
  "11011011",   // 70 F
  "11111101",   // 71 G
  "101010101",  // 72 H
  "1111111",    // 73 I
  "111111101",  // 74 J
  "101111101",  // 75 K
  "11010111",   // 76 L
  "10111011",   // 77 M
  "11011101",   // 78 N
  "10101011",   // 79 O
  "11010101",   // 80 P
  "111011101",  // 81 Q
  "10101111",   // 82 R
  "1101111",    // 83 S
  "1101101",    // 84 T
  "101010111",  // 85 U
  "110110101",  // 86 V
  "101011101",  // 87 W
  "101110101",  // 88 X
  "101111011",  // 89 Y
  "1010101101", // 90 Z
  "111110111",  // 91 [
  "111101111",  // 92 backslash
  "111111011",  // 93 ]
  "1010111111", // 94 ^
  "101101101",  // 95 _
  "1011011111", // 96 `
  "1011",       // 97 a
  "1011111",    // 98 b
  "101111",     // 99 c
  "101101",     // 100 d
  "11",         // 101 e
  "111101",     // 102 f
  "1011011",    // 103 g
  "101011",     // 104 h
  "1101",       // 105 i
  "111101011",  // 106 j
  "10111111",   // 107 k
  "11011",      // 108 l
  "111011",     // 109 m
  "1111",       // 110 n
  "111",        // 111 o
  "111111",     // 112 p
  "110111111",  // 113 q
  "10101",      // 114 r
  "10111",      // 115 s
  "101",        // 116 t
  "110111",     // 117 u
  "1111011",    // 118 v
  "1101011",    // 119 w
  "11011111",   // 120 x
  "1011101",    // 121 y
  "111010101",  // 122 z
  "1010110111", // 123 {
  "110111011",  // 124 |
  "1010110101", // 125 }
  "1011010111", // 126 ~
  EMPTY,        // 127 DEL
];

/// 由 Varicode 位串查 ASCII 字节。
#[must_use]
pub fn varicode_decode(bits: &str) -> Option<u8> {
  VARICODE.iter().position(|&v| v == bits).map(|i| i as u8)
}

/// 从位流解码文本（`true` = 位 1，`false` = 位 0；左位先）。
#[must_use]
pub fn decode_bits(bits: &[bool]) -> String {
  let mut out = String::new();
  let mut cur = String::new();
  let mut i = 0;
  while i < bits.len() {
    if bits[i] {
      cur.push('1');
      i += 1;
    } else if i + 1 < bits.len() && !bits[i + 1] {
      // "00" 分隔符：cur 是完整字符。
      if !cur.is_empty() {
        if let Some(c) = varicode_decode(&cur) {
          out.push(char::from(c));
        }
        cur.clear();
      }
      i += 2;
    } else {
      // 字符内部的单个 0。
      cur.push('0');
      i += 1;
    }
  }
  out
}

/// 把文本编码为 Varicode 位流（每个字符后跟 "00" 分隔符）。
#[must_use]
pub fn encode(text: &str) -> Vec<bool> {
  let mut bits = Vec::new();
  for ch in text.bytes() {
    let code = VARICODE.get(ch as usize).copied().unwrap_or("");
    for c in code.chars() {
      bits.push(c == '1');
    }
    bits.push(false);
    bits.push(false);
  }
  bits
}

/// 线性重采样到目标采样率。
fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
  if from == to || samples.len() < 2 {
    return samples.to_vec();
  }
  let ratio = f64::from(to) / f64::from(from);
  let n = (samples.len() as f64 * ratio).round() as usize;
  let mut out = Vec::with_capacity(n);
  for i in 0..n {
    let src = i as f64 / ratio;
    let idx = src as usize;
    let frac = (src - idx as f64) as f32;
    let a = samples[idx.min(samples.len() - 1)];
    let b = samples[(idx + 1).min(samples.len() - 1)];
    out.push(a + (b - a) * frac);
  }
  out
}

/// 生成低通 FIR 系数（sinc + Hamming 窗），`cutoff` 为归一化频率（0–0.5）。
fn lowpass_coeffs(cutoff: f32, taps: usize) -> Vec<f32> {
  let mut h = Vec::with_capacity(taps);
  let center = (taps - 1) as f32 / 2.0;
  let two_pi = std::f32::consts::TAU;
  for i in 0..taps {
    let x = i as f32 - center;
    let sinc = if x == 0.0 {
      2.0 * cutoff
    } else {
      (two_pi * cutoff * x).sin() / (std::f32::consts::PI * x)
    };
    let window = 0.54 - 0.46 * (two_pi * i as f32 / (taps - 1) as f32).cos();
    h.push(sinc * window);
  }
  let sum: f32 = h.iter().sum();
  for v in &mut h {
    *v /= sum;
  }
  h
}

/// FIR 卷积（直接型）。
fn apply_fir(samples: &[f32], coeffs: &[f32]) -> Vec<f32> {
  let mut out = vec![0.0; samples.len()];
  for i in 0..samples.len() {
    let mut acc = 0.0;
    for (k, &c) in coeffs.iter().enumerate() {
      if k > i {
        break;
      }
      acc += samples[i - k] * c;
    }
    out[i] = acc;
  }
  out
}

/// I/Q 下变频：把载波搬移到基带。
fn downconvert(samples: &[f32], center_hz: f32, rate: u32) -> (Vec<f32>, Vec<f32>) {
  let step = std::f32::consts::TAU * center_hz / rate as f32;
  let mut i = Vec::with_capacity(samples.len());
  let mut q = Vec::with_capacity(samples.len());
  let mut phase = 0.0f32;
  for &x in samples {
    let (s, c) = phase.sin_cos();
    i.push(x * c);
    q.push(x * s);
    phase = (phase + step) % std::f32::consts::TAU;
  }
  (i, q)
}

/// 从单声道音频样本解码 PSK31 文本。
///
/// `center_hz` 为音频载波频率（通常 1000 Hz），`sample_rate` 为输入采样率。
#[must_use]
pub fn decode_psk31(samples: &[f32], sample_rate: u32, center_hz: f32) -> String {
  if samples.len() < SYMBOL_LEN * 2 || sample_rate == 0 || center_hz <= 0.0 {
    return String::new();
  }
  let work = resample(samples, sample_rate, WORK_RATE);
  let (i, q) = downconvert(&work, center_hz, WORK_RATE);
  let coeffs = lowpass_coeffs(BAUD as f32 / WORK_RATE as f32, 256);
  let i_lp = apply_fir(&i, &coeffs);
  let q_lp = apply_fir(&q, &coeffs);

  // 符号采样：补偿 FIR 群延迟，对每个符号周期积分（匹配滤波，提高抗噪）。
  let delay = (coeffs.len() - 1) / 2;
  let mut symbols: Vec<(f32, f32)> = Vec::new();
  let mut k = 0usize;
  loop {
    let start = k * SYMBOL_LEN + delay;
    if start >= i_lp.len() {
      break;
    }
    let end = (start + SYMBOL_LEN).min(i_lp.len());
    let mut sum_i = 0.0;
    let mut sum_q = 0.0;
    for j in start..end {
      sum_i += i_lp[j];
      sum_q += q_lp[j];
    }
    symbols.push((sum_i, sum_q));
    k += 1;
  }

  // 差分位判决：相邻符号点积为正（相位差 < 90°）视为位 1，否则位 0。
  let mut bits = Vec::with_capacity(symbols.len().saturating_sub(1));
  for k in 1..symbols.len() {
    let (i0, q0) = symbols[k - 1];
    let (i1, q1) = symbols[k];
    bits.push(i0 * i1 + q0 * q1 >= 0.0);
  }
  decode_bits(&bits)
}

/// 频偏搜索范围（±Hz）。
const SCAN_HALF: f32 = 40.0;
/// 频偏搜索步进（Hz）。
const SCAN_STEP: f32 = 4.0;

/// 评分：解码出的可打印字符数（用于从候选频率中挑选最优）。
fn score(text: &str) -> usize {
  text
    .chars()
    .filter(|c| c.is_ascii_graphic() || *c == ' ')
    .count()
}

/// 自动频偏搜索解码：在 `center_hz` 附近扫描候选载波频率，取结果最优者。
///
/// 真实短波信号常有几到几十 Hz 的载波频偏，固定频率解码会失效；此函数在
/// `center_hz ± SCAN_HALF` 范围内按 `SCAN_STEP` 步进搜索。
#[must_use]
pub fn decode_psk31_auto(samples: &[f32], sample_rate: u32, center_hz: f32) -> String {
  let mut best = String::new();
  let mut best_score = 0usize;
  let steps = (SCAN_HALF / SCAN_STEP).round() as i32;
  for i in -steps..=steps {
    let f = center_hz + i as f32 * SCAN_STEP;
    let text = decode_psk31(samples, sample_rate, f);
    let s = score(&text);
    if s > best_score {
      best_score = s;
      best = text;
    }
  }
  best
}

/// 简单 LCG 伪随机数生成器（0..1）。
fn lcg(seed: u64) -> impl FnMut() -> f64 {
  let mut s = seed;
  move || {
    s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    ((s >> 11) as f64) / (1u64 << 53) as f64
  }
}

/// 标准正态分布随机数（Box–Muller）。
fn gauss(rng: &mut impl FnMut() -> f64) -> f32 {
  let u1 = rng().max(f64::MIN_POSITIVE);
  let u2 = rng();
  ((-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()) as f32
}

/// 差分 BPSK 调制：前导一个参考符号，位 0 相位反转、位 1 相位保持。
///
/// 用浮点符号边界，避免非整数倍采样率（如 44100 Hz）下整数取整累积误差。
fn modulate_bpsk(bits: &[bool], rate: u32, center: f32, noise_sigma: f32) -> Vec<f32> {
  let samples_per_symbol = f64::from(rate) / BAUD;
  let total = ((bits.len() + 1) as f64 * samples_per_symbol).round() as usize;
  let step = std::f32::consts::TAU * center / rate as f32;
  let mut phases = Vec::with_capacity(bits.len() + 1);
  let mut p = 0.0f32;
  phases.push(p);
  for &bit in bits {
    if !bit {
      p = (p + std::f32::consts::PI) % std::f32::consts::TAU;
    }
    phases.push(p);
  }
  let mut samples = Vec::with_capacity(total);
  let mut t = 0.0f32;
  for n in 0..total {
    let sym = (n as f64 / samples_per_symbol).floor() as usize;
    let phase = phases[sym.min(phases.len() - 1)];
    samples.push((t + phase).cos());
    t = (t + step) % std::f32::consts::TAU;
  }
  if noise_sigma > 0.0 {
    let mut rng = lcg(1);
    for s in &mut samples {
      *s += gauss(&mut rng) * noise_sigma;
    }
  }
  samples
}

/// 合成一段 PSK31 音频信号（用于测试与测试数据生成）。
///
/// `center_hz` 为载波频率，`noise_sigma` 为加性高斯白噪声标准差（0 表示不加噪）。
#[must_use]
pub fn synthesize(text: &str, rate: u32, center_hz: f32, noise_sigma: f32) -> Vec<f32> {
  modulate_bpsk(&encode(text), rate, center_hz, noise_sigma)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn varicode_roundtrips_printable_chars() {
    for (i, code) in VARICODE.iter().enumerate().skip(32).take(95) {
      assert_eq!(
        varicode_decode(code),
        Some(i as u8),
        "char {i} ({})",
        i as u8 as char
      );
    }
    assert_eq!(varicode_decode("10101101"), Some(b'C'));
    assert_eq!(varicode_decode("111011101"), Some(b'Q'));
    assert_eq!(varicode_decode("1"), Some(b' '));
    assert_eq!(varicode_decode("1010101101"), Some(b'Z'));
  }

  #[test]
  fn decode_bits_splits_on_double_zero() {
    // "CQ" = 10101101 00 111011101 00
    let bits = "10101101"
      .bytes()
      .chain("00".bytes())
      .chain("111011101".bytes())
      .chain("00".bytes())
      .map(|b| b == b'1')
      .collect::<Vec<_>>();
    assert_eq!(decode_bits(&bits), "CQ");
  }

  #[test]
  fn decodes_synthetic_psk31() {
    let text = "CQ TEST";
    let samples = synthesize(text, 8000, 1000.0, 0.0);
    assert_eq!(decode_psk31(&samples, 8000, 1000.0), text);
  }

  #[test]
  fn decodes_at_other_sample_rate() {
    // 用 44100 Hz 合成再解码，验证重采样路径。
    let text = "HELLO";
    let samples = synthesize(text, 44100, 1000.0, 0.0);
    assert_eq!(decode_psk31(&samples, 44100, 1000.0), text);
  }

  #[test]
  fn tolerates_carrier_offset_via_auto_scan() {
    // +15 Hz 频偏：固定频率解码会失败，auto 搜索应能解出。
    let text = "CQ DX";
    let samples = synthesize(text, 8000, 1015.0, 0.0);
    assert_eq!(decode_psk31_auto(&samples, 8000, 1000.0), text);
  }

  #[test]
  fn tolerates_moderate_noise() {
    // 加高斯噪声（σ=0.2，相对单位幅度信号约 14 dB SNR），符号积分应能解出。
    let text = "PSK31";
    let samples = synthesize(text, 8000, 1000.0, 0.2);
    assert_eq!(decode_psk31(&samples, 8000, 1000.0), text);
  }
}
