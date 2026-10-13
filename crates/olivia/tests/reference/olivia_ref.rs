//! Olivia MFSK **参考实现**（互操作对拍用，独立于 `src/`）。
//!
//! 转写自 sntfrc/olivia-python（MIT License，© 2020 Federico Santandrea，
//! 与 fldigi 对比验证过的独立实现）—— 原为纯 Python 移植，现改为 Rust 版本
//! （与 crate 核心**不共享代码**，只共享上游出处；双实现双向对拍才有意义）。
//!
//! 参数：64 位固定密钥 `0xE257E6D0291574EC`、Walsh-Hadamard 扩频、符号 = spb
//! 位交织、Gray 码、每符号一个音调（fsep = bandwidth/symbols = 31.25 Hz）。
//! 注意 `decode` 的离线对齐策略与在线参考不同（取零存疑且总置信度最高的对齐），
//! 门限逻辑本身保持原样 —— 见原 Python 版注释。

use std::f64::consts::PI;

/// 参考实现的采样率（8 kHz）。
pub const SAMPLE_RATE: u32 = 8000;
/// Olivia 规范的 64 位固定密钥。
pub const KEY: u64 = 0xE257E6D0291574EC;
/// 块译码的峰值门限（fldigi/olivia-python 同款）。
pub const BLOCK_THRESHOLD: f64 = 24.0;

/// Gray 编码。
#[must_use]
pub fn gray(n: u32) -> u32 {
  n ^ (n >> 1)
}

/// Gray 解码。
#[must_use]
pub fn degray(n: u32) -> u32 {
  let mut mask = n;
  let mut out = n;
  while mask != 0 {
    mask >>= 1;
    out ^= mask;
  }
  out
}

/// 参考实现的逆 Walsh-Hadamard（非归一化，原地）。
pub fn ifwht(data: &mut [f64]) {
  let n = data.len();
  let mut step = n / 2;
  while step >= 1 {
    let mut ptr = 0;
    while ptr < n {
      for ptr2 in ptr..ptr + step {
        let b1 = data[ptr2];
        let b2 = data[ptr2 + step];
        data[ptr2] = b1 - b2;
        data[ptr2 + step] = b1 + b2;
      }
      ptr += 2 * step;
    }
    step /= 2;
  }
}

/// 参考实现的 Walsh-Hadamard（非归一化，原地）。
pub fn fwht(data: &mut [f64]) {
  let n = data.len();
  let mut step = 1;
  while step < n {
    let mut ptr = 0;
    while ptr < n {
      for ptr2 in ptr..ptr + step {
        let b1 = data[ptr2];
        let b2 = data[ptr2 + step];
        data[ptr2] = b2 + b1;
        data[ptr2 + step] = b2 - b1;
      }
      ptr += 2 * step;
    }
    step *= 2;
  }
}

/// 单频 Goertzel 功率。
pub fn goertzel(xs: &[f32], freq: f64, rate: u32) -> f64 {
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

/// spb 个字符 → 64 个符号值（每个符号 spb 位）。
#[must_use]
pub fn prepare_symbols(chars: &[char], spb: usize) -> Vec<u32> {
  let mut w = vec![[0.0f64; 64]; spb];
  for (i, &ch) in chars.iter().enumerate() {
    let mut q = u32::from(ch);
    if q > 127 {
      q = 0;
    }
    if q < 64 {
      w[i][q as usize] = 1.0;
    } else {
      w[i][(q - 64) as usize] = -1.0;
    }
    ifwht(&mut w[i]);
    for (j, slot) in w[i].iter_mut().enumerate() {
      if (KEY >> ((j + 13 * i) % 64)) & 1 == 1 {
        *slot = -*slot;
      }
    }
  }
  let mut syms = vec![0u32; 64];
  for (sym, slot) in syms.iter_mut().enumerate() {
    for (k, bit) in (0..spb)
      .map(|k| {
        let q = (k as isize - sym as isize).rem_euclid(spb as isize) as usize;
        u32::from(w[q][sym] < 0.0)
      })
      .enumerate()
    {
      *slot |= bit << k;
    }
  }
  syms
}

/// 文本 → 音频样本（与参考实现的块结构一致：尾部 + 数据块，preamble 右对齐）。
#[must_use]
pub fn synthesize(text: &str, symbols: u32, bandwidth: f64, center: f64) -> Vec<f32> {
  let spb = f64::from(symbols).log2() as usize;
  let fsep = bandwidth / f64::from(symbols);
  let wlen = (f64::from(SAMPLE_RATE) / fsep).ceil() as usize;

  let first_freq = center - bandwidth / 2.0 + fsep / 2.0;
  let last_freq = center + bandwidth / 2.0 - fsep / 2.0;

  // 固定频率的纯音（dur 秒）。
  let tone_at = |freq: f64, dur: f64| -> Vec<f32> {
    let n = (dur * f64::from(SAMPLE_RATE)) as usize;
    (0..n)
      .map(|i| ((2.0 * PI * freq * i as f64 / f64::from(SAMPLE_RATE)).sin()) as f32)
      .collect()
  };
  // 音调 number：2/fsep 秒，带规范里的余弦成形窗。
  let tone = |number: u32| -> Vec<f32> {
    let freq = center - bandwidth / 2.0 + fsep / 2.0 + fsep * f64::from(number);
    let mut out: Vec<f32> = (0..2 * wlen)
      .map(|i| ((2.0 * PI * freq * i as f64 / f64::from(SAMPLE_RATE)).sin()) as f32)
      .collect();
    let n = out.len();
    for (i, s) in out.iter_mut().enumerate() {
      let x = -PI + 2.0 * PI * i as f64 / n as f64;
      let shape = 1.0 + 1.191_378_572_3 * x.cos()
        - 0.079_301_855_8 * (2.0 * x).cos()
        - 0.217_144_202_6 * (3.0 * x).cos()
        - 0.001_452_607_6 * (4.0 * x).cos();
      *s = (f64::from(*s) * shape) as f32;
    }
    out
  };

  // 首尾：最低 / 最高音调交替，各 1/4 秒。
  let mut tail = tone_at(first_freq, 0.25);
  tail.extend(tone_at(last_freq, 0.25));
  tail.extend(tone_at(first_freq, 0.25));
  tail.extend(tone_at(last_freq, 0.25));

  // preamble 是完整的 64 符号块，tail 右对齐（数据从符号边界开始）。
  // 饱和运算：非标准子模式下 `wlen` 可能小于 1/4 秒样本数（`4/500@750` 时 wlen=64），
  // 直接相减会下溢（debug panic / release 天量分配）；此时 preamble 退化为空。
  let mut audio = vec![0.0f32; 64usize.saturating_mul(wlen).saturating_sub(tail.len())];
  audio.extend(tail);

  let chars: Vec<char> = text.chars().collect();
  for chunk in chars.chunks(spb) {
    let mut piece: Vec<char> = chunk.to_vec();
    while piece.len() < spb {
      piece.push('\0');
    }
    for s in prepare_symbols(&piece, spb) {
      let t = tone(gray(s));
      audio.extend(&t[..wlen]);
    }
  }
  audio
}

/// 音频样本 → 文本（离线对齐：取零存疑且总置信度最高的对齐，见模块注释）。
#[must_use]
pub fn decode(samples: &[f32], symbols: u32, bandwidth: f64, center: f64) -> String {
  let spb = f64::from(symbols).log2() as usize;
  let fsep = bandwidth / f64::from(symbols);
  let wlen = (f64::from(SAMPLE_RATE) / fsep).ceil() as usize;
  let base = center - bandwidth / 2.0 + fsep / 2.0;

  // 每符号：在各音调 bin 上取最大 → degray → 符号值。
  let mut syms = Vec::new();
  let mut i = 0usize;
  // `checked_add`：`fsep` 极小时 `wlen` 会饱和成 `usize::MAX`，直接 `i + wlen` 回绕
  // 会让循环语义错误（debug panic / release 死循环）。
  while i.checked_add(wlen).is_some_and(|end| end <= samples.len()) {
    let win = &samples[i..i + wlen];
    let mut best = (0usize, f64::NEG_INFINITY);
    for m in 0..symbols as usize {
      let p = goertzel(win, base + fsep * m as f64, SAMPLE_RATE);
      if p > best.1 {
        best = (m, p);
      }
    }
    syms.push(degray(best.0 as u32));
    i += wlen;
  }
  if syms.len() < 64 {
    return String::new();
  }

  let mut text = String::new();
  let edge = degray(symbols - 1);
  let preamble = [(34..40, 0), (42..48, edge), (50..56, 0), (58..64, edge)]
    .iter()
    .all(|(r, tone)| syms[r.clone()].iter().all(|s| s == tone));
  let mut start = if preamble { 64 } else { 0 };
  while start + 64 <= syms.len() {
    let mut best: Option<(f64, usize, Vec<u16>)> = None;
    let end = (start + 63).min(syms.len() - 64);
    for s in start..=end {
      let block = &syms[s..s + 64];
      let mut w = vec![[0.0f64; 64]; spb];
      let mut doubt = 0;
      let mut total = 0.0f64;
      let mut out_chars = Vec::with_capacity(spb);
      for (i2, row) in w.iter_mut().enumerate() {
        for (j, slot) in row.iter_mut().enumerate() {
          let bit = (block[j] >> ((i2 + j) % spb)) & 1;
          *slot = if bit == 1 { -1.0 } else { 1.0 };
        }
        for (j, slot) in row.iter_mut().enumerate() {
          if (KEY >> ((j + 13 * i2) % 64)) & 1 == 1 {
            *slot = -*slot;
          }
        }
        fwht(row);
        let mut c = 0usize;
        for k in 1..64 {
          if row[k].abs() > row[c].abs() {
            c = k;
          }
        }
        total += row[c].abs();
        if row[c].abs() < BLOCK_THRESHOLD {
          doubt += 1;
        }
        out_chars.push(if row[c] < 0.0 {
          (c + 64) as u16
        } else {
          c as u16
        });
      }
      if doubt == 0 && best.as_ref().is_none_or(|(t, _, _)| total > *t) {
        best = Some((total, s, out_chars));
      }
    }
    let Some((_, s, out_chars)) = best else {
      break;
    };
    for &c in &out_chars {
      if c != 0 {
        text.push(char::from(c as u8));
      }
    }
    start = s + 64;
  }
  text
}
