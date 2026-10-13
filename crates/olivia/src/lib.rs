//! Olivia 数字模式：MFSK + Walsh-Hadamard 扩频 + 密钥交织。
//!
//! 算法 1:1 转写自 sntfrc/olivia-python（MIT License，© 2020 Federico Santandrea，
//! 与 fldigi 对比验证过的独立实现）：https://github.com/sntfrc/olivia-python
//!
//! 结构（默认 32/1000）：
//! - 32 个音调、1000 Hz 带宽、31.25 波特，每个符号 32 ms（8000 Hz 采样下 256 样本）；
//! - 每块 = 64 符号 = `spb = log2(音调数)` 个字符；字符（7 位 ASCII，0–127）映射为
//!   64 维脉冲 → 逆 Walsh-Hadamard → 与 64 位固定密钥 `0xE257E6D0291574EC` 逐字符
//!   旋转异或 → 按位交织到 64 个符号（每符号 spb 位）→ Gray 码 → 音调序号；
//! - 解码 = 上述过程的逆：逐符号 Goertzel 判音调 → degray → 去交织 → 异或 → WHT →
//!   argmax |幅值|（门限 24，参考实现的 `BLOCK_THRESHOLD`）。
//!
//! # 验证
//!
//! - 单测：往返（8/250 与 32/1000）、错位起始、零噪声/含噪；
//! - **互操作测试**（`tests/reference_interop.rs`）：调用 `tests/reference/olivia_ref.rs`
//!   里的参考实现（上游 Python 的 Rust 移植，与本 crate 的实现相互独立），**双向对拍** ——
//!   参考实现编码 → 本 crate 解码；本 crate 编码 → 参考实现解码。
//!
//! # 与参考移植的已知差异
//!
//! `tests/reference/olivia_ref.rs` 在合成数据音调时套了 4 项余弦成形窗，本 crate 的
//! [`encode_audio`] 用裸正弦：成形只改变频谱旁瓣、不影响解调，但合成样本因此不是
//! 「发射机等价」的 —— 往返与互锁测试仍有效，频谱测量不要用它。

use ham_web_dsp::tone_power;

pub mod walsh;

use walsh::{KEY, degray, fwht, gray, ifwht};

/// 符号速率：所有标准子模式共用 31.25 波特（带宽 = 音调数 × 31.25 Hz）。
pub const BAUD: f64 = 31.25;
/// 每块符号数。
pub const BLOCK_SYMBOLS: usize = 64;
/// 参考实现的 WHT 峰值门限：`|峰值| < 24` 记为「存疑」，整块存疑则拒收。
pub const BLOCK_THRESHOLD: f64 = 24.0;

/// 支持的子模式参数：`(音调数, 带宽 Hz, 默认中心频率 Hz)`。
pub const SUBMODES: &[(&str, u32, f64, f64)] = &[
  ("4/125", 4, 125.0, 750.0),
  ("8/250", 8, 250.0, 750.0),
  ("16/500", 16, 500.0, 1000.0),
  ("32/1000", 32, 1000.0, 1500.0),
];

/// 解析 `"32/1000@1500"` 形式的子模式参数。
#[must_use]
pub fn parse_submode(spec: &str) -> Option<(u32, f64, f64)> {
  let (syms_bw, center) = spec.rsplit_once('@')?;
  let (syms, bw) = syms_bw.split_once('/')?;
  Some((syms.parse().ok()?, bw.parse().ok()?, center.parse().ok()?))
}

/// 音调数 → 每符号位数（4/8/16/32 → 2/3/4/5）。
#[must_use]
pub fn bits_per_symbol(symbols: u32) -> Option<u32> {
  match symbols {
    4 => Some(2),
    8 => Some(3),
    16 => Some(4),
    32 => Some(5),
    _ => None,
  }
}

/// 文本 → 64 符号块（`\0` 填充到整块）。
fn text_to_blocks(text: &str, spb: usize) -> Vec<Vec<u8>> {
  let mut blocks = Vec::new();
  let mut chars = Vec::new();
  for ch in text.chars() {
    chars.push(if (ch as u32) <= 127 { ch as u8 } else { 0 });
  }
  for chunk in chars.chunks(spb) {
    let mut block = chunk.to_vec();
    while block.len() < spb {
      block.push(0);
    }
    blocks.push(block);
  }
  blocks
}

/// spb 个字符 → 64 个符号值（每符号 spb 位）。
///
/// 前置条件（调用方 [`text_to_blocks`] 保证）：`chars.len() >= spb`，且每个字符 < 128。
///
/// 违反前置条件时返回静音块而不是越界切片 / 越界索引：这是私有函数，但 release 下
/// `debug_assert` 不生效，将来多一个调用方就会变成 wasm trap。
fn encode_block(chars: &[u8], spb: usize) -> [u32; BLOCK_SYMBOLS] {
  debug_assert!(chars.len() >= spb, "字符数必须覆盖 spb");
  debug_assert!(chars.iter().all(|&c| c < 128), "字符必须 < 128");
  if chars.len() < spb || chars[..spb].iter().any(|&c| c >= 128) {
    return [0u32; BLOCK_SYMBOLS];
  }
  let mut w = vec![vec![0.0f64; BLOCK_SYMBOLS]; spb];
  for i in 0..spb {
    let q = chars[i] as usize;
    if q < 64 {
      w[i][q] = 1.0;
    } else {
      w[i][q - 64] = -1.0;
    }
    ifwht(&mut w[i]);
    for (j, slot) in w[i].iter_mut().enumerate() {
      if key_bit((j + 13 * i) % 64) == 1 {
        *slot = -*slot;
      }
    }
  }
  let mut syms = [0u32; BLOCK_SYMBOLS];
  for (sym, slot) in syms.iter_mut().enumerate() {
    for bis in 0..spb {
      let q = (bis + spb - (sym % spb)) % spb;
      if w[q][sym] < 0.0 {
        *slot |= 1 << bis;
      }
    }
  }
  syms
}

/// 密钥 `0xE257E6D0291574EC` 的第 i 位（LSB 起）。
#[must_use]
pub fn key_bit(i: usize) -> u8 {
  ((KEY >> (i & 63)) & 1) as u8
}

/// `encode_audio` 输出样本总数上限（5×10⁷ ≈ 200 MB f32）：`sample_rate = u32::MAX` 时
/// 单个 64 符号窗口就有数十亿样本，preamble 分配会直接 OOM。
const MAX_ENCODE_SAMPLES: usize = 50_000_000;

/// 文本 → 音频样本（8 kHz；preamble = 静音 + 首尾音调交替尾，与参考实现一致）。
#[must_use]
pub fn encode_audio(
  text: &str,
  symbols: u32,
  bandwidth: f64,
  center: f64,
  sample_rate: u32,
) -> Option<Vec<f32>> {
  let spb = bits_per_symbol(symbols)? as usize;
  if sample_rate == 0 || !bandwidth.is_finite() || bandwidth <= 0.0 || !center.is_finite() {
    return None;
  }
  let fsep = bandwidth / f64::from(symbols);
  let wlen = (f64::from(sample_rate) / fsep).ceil() as usize;
  // preamble 静音 = BLOCK_SYMBOLS×wlen − sample_rate，两个方向都要挡：
  // - fsep 过大（如子模式 "4/500@750" 的 125 Hz）→ wlen 过小，减法 usize 下溢
  //   （debug panic、release 分配天文数字内存 OOM）；
  // - fsep 过小（bandwidth 极小）→ wlen 大到 `BLOCK_SYMBOLS×wlen` 也要几十 GB。
  // 标准子模式的 fsep 恒为 31.25 Hz（wlen ≈ rate/31.25），两个守卫都碰不到。
  if wlen == 0 || wlen > sample_rate as usize / 4 || BLOCK_SYMBOLS * wlen < sample_rate as usize {
    return None;
  }
  // 预估总样本数（preamble 一个窗口 + 数据块），先拒绝再分配。
  let blocks = text.chars().count().div_ceil(spb);
  if blocks
    .saturating_add(1)
    .saturating_mul(BLOCK_SYMBOLS.saturating_mul(wlen))
    > MAX_ENCODE_SAMPLES
  {
    return None;
  }
  let mut out = Vec::new();
  let first_freq = center - bandwidth / 2.0 + fsep / 2.0;
  let last_freq = center + bandwidth / 2.0 - fsep / 2.0;
  // preamble：64 符号块，尾部 4×0.25 秒首尾音调交替，前面静音。
  out.extend(std::iter::repeat_n(
    0.0f32,
    BLOCK_SYMBOLS * wlen - sample_rate as usize,
  ));
  for (f, _) in [
    (first_freq, 0),
    (last_freq, 1),
    (first_freq, 0),
    (last_freq, 1),
  ] {
    for i in 0..sample_rate as usize / 4 {
      out.push((2.0 * std::f64::consts::PI * f * i as f64 / f64::from(sample_rate)).sin() as f32);
    }
  }
  // 数据块
  for block in text_to_blocks(text, spb) {
    for sym in encode_block(&block, spb) {
      let tone = first_freq + fsep * f64::from(gray(sym));
      for _ in 0..wlen {
        out.push(
          (2.0 * std::f64::consts::PI * tone * (out.len()) as f64 / f64::from(sample_rate)).sin()
            as f32,
        );
      }
    }
  }
  Some(out)
}

/// 音频 → 解调出的符号序列（逐符号 Goertzel 判音调 → degray）。
#[must_use]
pub fn demodulate(
  samples: &[f32],
  rate: u32,
  symbols: u32,
  bandwidth: f64,
  center: f64,
) -> Vec<u32> {
  // rate=0 / symbols=0 / 非正带宽 → wlen=0 → `i += 0` 死循环；公共 API 防住。
  // `symbols` 只接受标准音调数（4/8/16/32）：`4e9` 会让每个窗口跑数十亿次 Goertzel，
  // 等于永久卡死。
  if rate == 0
    || bits_per_symbol(symbols).is_none()
    || samples.is_empty()
    || !bandwidth.is_finite()
    || bandwidth <= 0.0
    || !center.is_finite()
  {
    return Vec::new();
  }
  let fsep = bandwidth / f64::from(symbols);
  let wlen = (f64::from(rate) / fsep).ceil() as usize;
  if wlen == 0 {
    return Vec::new();
  }
  let base = center - bandwidth / 2.0 + fsep / 2.0;
  let mut out = Vec::new();
  // 剩余长度比较而不是 `i + wlen`：`bandwidth` 极小时 `wlen` 饱和成 `usize::MAX`，加法会回绕。
  let mut rest = samples;
  while rest.len() >= wlen {
    let (win, tail) = rest.split_at(wlen);
    rest = tail;
    let mut best = (0usize, f64::NEG_INFINITY);
    for m in 0..symbols as usize {
      let p = tone_power(win, rate, base + fsep * m as f64);
      if p > best.1 {
        best = (m, p);
      }
    }
    out.push(degray(best.0 as u32));
  }
  out
}

/// 64 个符号 → 字符（`(字符码, 总置信度, 存疑字符数)`）。
fn decode_block(syms: &[u32], spb: usize) -> (Vec<u8>, f64, usize) {
  let mut chars = Vec::with_capacity(spb);
  let mut total = 0.0f64;
  let mut doubt = 0usize;
  for i in 0..spb {
    // 栈数组：BLOCK_SYMBOLS 恒 64，省去每字符一次堆分配（长音频解码可达
    // 数十万次调用）。
    let mut w = [0.0f64; BLOCK_SYMBOLS];
    for (j, slot) in w.iter_mut().enumerate() {
      let bit = (syms[j] >> ((i + j) % spb)) & 1;
      *slot = if bit == 1 { -1.0 } else { 1.0 };
    }
    for (j, slot) in w.iter_mut().enumerate() {
      if key_bit((j + 13 * i) % 64) == 1 {
        *slot = -*slot;
      }
    }
    fwht(&mut w);
    let mut c = 0usize;
    let mut peak = 0.0f64;
    for (k, &v) in w.iter().enumerate() {
      if v.abs() > peak {
        peak = v.abs();
        c = k;
      }
    }
    total += peak;
    if peak < BLOCK_THRESHOLD {
      doubt += 1;
    }
    chars.push(if w[c] < 0.0 { (c + 64) as u8 } else { c as u8 });
  }
  (chars, total, doubt)
}

/// 完整解码：符号流 → 文本。
///
/// 失锁时只在当前一块宽的局部窗口内选择对齐，避免后面的完美块盖掉可纠正的前文。
/// 锁定后按 64 符号顺序推进；当前块低于门限才重新同步，扫描工作量随流长度线性增长。
#[must_use]
pub fn decode_symbols(syms: &[u32], spb: usize) -> String {
  decode_symbols_run(syms, spb).0
}

/// 同 [`decode_symbols`]，另返回**最长的连续锁定块数**。
///
/// 噪声里「偶然有一块超过 WHT 门限」并不罕见（对齐搜索在 64 个偏移里挑最好的，窄带子模式
/// 下每块只有 2 ~ 3 位冗余），但**连续**几块都过门限就几乎不可能 —— 真信号才会连成片。
/// 调用方据此区分「真报文」与「噪声里碰出的几个乱码字符」。
#[must_use]
fn decode_symbols_run(syms: &[u32], spb: usize) -> (String, usize) {
  if !(2..=5).contains(&spb) {
    return (String::new(), 0);
  }
  let mut text = String::new();
  let mut start = 0usize;
  let mut locked = false;
  let (mut run, mut longest) = (0usize, 0usize);
  while start + BLOCK_SYMBOLS <= syms.len() {
    let (chars, _, doubt) = decode_block(&syms[start..start + BLOCK_SYMBOLS], spb);
    let (s, chars) = if locked && doubt == 0 {
      // 原位继续锁定：这才是「连成片」的证据，run 只在这里累加。
      run += 1;
      (start, chars)
    } else {
      let end = (start + BLOCK_SYMBOLS - 1).min(syms.len() - BLOCK_SYMBOLS);
      let mut best: Option<(f64, usize, Vec<u8>)> = None;
      for s in start..=end {
        let (chars, total, doubt) = decode_block(&syms[s..s + BLOCK_SYMBOLS], spb);
        if doubt == 0 && best.as_ref().is_none_or(|(t, _, _)| total > *t) {
          best = Some((total, s, chars));
        }
      }
      let Some((_, s, chars)) = best else {
        locked = false;
        run = 0;
        start += BLOCK_SYMBOLS;
        continue;
      };
      // 靠对齐搜索找到的块：在 64 个偏移里挑最好的，噪声里几乎总能碰出一块，所以它只算
      // 新区间的第一块，不能算「继续锁定」。
      run = 1;
      (s, chars)
    };
    for c in chars {
      if c != 0 {
        text.push(char::from(c));
      }
    }
    locked = true;
    longest = longest.max(run);
    start = s + BLOCK_SYMBOLS;
  }
  (text, longest)
}

/// 线性重采样：把时钟误差 `ppm` 补偿掉（`ppm > 0` 表示录音的采样时钟比发射端慢，符号被拉长）。
fn resample_ppm(samples: &[f32], ppm: f64) -> Vec<f32> {
  let f = 1.0 + ppm * 1e-6;
  let n = (samples.len() as f64 / f) as usize;
  (0..n)
    .map(|i| {
      let x = i as f64 * f;
      let i0 = x as usize;
      let frac = (x - i0 as f64) as f32;
      samples[i0] * (1.0 - frac) + samples.get(i0 + 1).copied().unwrap_or(0.0) * frac
    })
    .collect()
}

/// 时钟误差候选（ppm）：±800，步长 100，共 17 个。
const DRIFT_CANDIDATES: [f64; 17] = [
  0.0, 100.0, -100.0, 200.0, -200.0, 300.0, -300.0, 400.0, -400.0, 500.0, -500.0, 600.0, -600.0,
  700.0, -700.0, 800.0, -800.0,
];

/// 同 [`decode_audio`]，但会**扫描时钟误差**：在 ±800 ppm 内试 17 个重采样候选，取输出最长者
/// （并列取 |ppm| 最小的，所以不漂移的录音选 0）。返回 `(文本, 选中的 ppm)`。
///
/// Olivia 没有符号定时恢复，窗口固定从第 0 个样本按标称符号长度推进：发射机与声卡时钟差几百
/// ppm 时，长报文后半段的窗口会逐渐错位，丢几个字符。实测 440 字符 / 8 ~ 5 分钟的报文：
/// 100 ppm 约 0.99、300 ppm 约 0.75 / 0.50、1000 ppm 约 0.49 / 0.38（32/1000 与 8/250）；
/// 扫描后在噪声幅度 0 ~ 2 下 32/1000 恢复到 440 / 440，8/250 恢复到 438 ~ 440 / 440，
/// 不漂移时选 0 ppm 不动。
///
/// **代价是耗时约 ×14 ~ ×19**（17 次解码 + 17 次重采样；原生 release 下 166 s 音频 1.1 s），所以
/// 它是**独立入口、不是 [`decode_audio`] 的默认行为** —— 典型声卡误差只有 20 ~ 100 ppm，
/// 绝大多数录音不需要。页面目前没有接线；要用的话建议先正常解一次，只有长报文才补扫。
#[must_use]
pub fn decode_audio_drift_search(
  samples: &[f32],
  rate: u32,
  symbols: u32,
  bandwidth: f64,
  center: f64,
) -> Option<(String, f64)> {
  let mut best: Option<(String, f64)> = None;
  for ppm in DRIFT_CANDIDATES {
    let fixed;
    let input = if ppm == 0.0 {
      samples
    } else {
      fixed = resample_ppm(samples, -ppm);
      &fixed
    };
    if let Some(text) = decode_audio(input, rate, symbols, bandwidth, center) {
      // 严格大于才替换：候选按 |ppm| 从小到大排，并列时保留更接近 0 的。
      if best
        .as_ref()
        .is_none_or(|(b, _)| text.chars().count() > b.chars().count())
      {
        best = Some((text, ppm));
      }
    }
  }
  best
}

/// 没有前导时，至少要连续锁定这么多块才认为是真信号（见 [`decode_symbols_run`]）。
const MIN_RUN_WITHOUT_PREAMBLE: usize = 3;

/// 便捷入口：音频 → 文本（自动解析子模式）。
#[must_use]
pub fn decode_audio(
  samples: &[f32],
  rate: u32,
  symbols: u32,
  bandwidth: f64,
  center: f64,
) -> Option<String> {
  let spb = bits_per_symbol(symbols)? as usize;
  let syms = demodulate(samples, rate, symbols, bandwidth, center);
  // 标准前导的首尾音调交替不是数据：低音调数下它也能偶然超过 WHT 门限。
  // 只识别前导四段的内部符号，避开 0.25 秒边界处的混合窗口。
  let last = degray(symbols - 1);
  let has_preamble = syms.len() >= BLOCK_SYMBOLS
    && [(34..40, 0), (42..48, last), (50..56, 0), (58..64, last)]
      .iter()
      .all(|(range, tone)| syms[range.clone()].iter().all(|s| s == tone));
  let data = if has_preamble {
    &syms[BLOCK_SYMBOLS..]
  } else {
    &syms
  };
  if data.is_empty() {
    return None;
  }
  // 噪声 / 静音解不出任何字符时返回 `None`，与 DominoEX / RTTY 的语义一致，
  // 不靠调用方再 `.filter(!is_empty)` 兜底。
  let (text, longest_run) = decode_symbols_run(data, spb);
  // 有前导：它本身就是强证据，短报文（一两块）照常放行。
  // 没有前导：必须连续锁定足够多块，否则只是噪声里碰出的几个字符。
  // 如实说明：没有前导的**极短**报文（不足 3 块，32/1000 约 15 字符）会被拒收 ——
  // 与噪声在统计上无法区分，宁可漏判也不把乱码当成解码结果。
  (!text.is_empty() && (has_preamble || longest_run >= MIN_RUN_WITHOUT_PREAMBLE)).then_some(text)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 高斯白噪声（xorshift64 + Box–Muller）。线性同余的低位相关会给音调判决带来偏置，
  /// 测不出真实白噪声下的误报率。
  fn white_noise(seed: u64, n: usize) -> Vec<f32> {
    let mut st = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
      st ^= st << 13;
      st ^= st >> 7;
      st ^= st << 17;
      (st >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
      .map(|_| {
        let (u1, u2) = (next().max(1e-12), next());
        ((-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * 0.35) as f32
      })
      .collect()
  }

  const MODES: [(u32, f64, f64); 4] = [
    (32, 1000.0, 1500.0),
    (16, 500.0, 1000.0),
    (8, 250.0, 750.0),
    (4, 125.0, 750.0),
  ];

  #[test]
  fn drift_search_recovers_a_long_message_under_clock_error_and_noise() {
    // 440 字符报文 + 300 / -300 / 600 ppm 时钟误差 + 白噪声：普通解码会丢尾部若干字符，
    // 扫描后应基本还原。用最长公共子序列衡量，容忍个别字符。
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 ".repeat(8);
    let lcs = |a: &str, b: &str| -> usize {
      let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
      let mut row = vec![0usize; b.len() + 1];
      for &x in &a {
        let mut d = 0;
        for (j, &y) in b.iter().enumerate() {
          let up = row[j + 1];
          row[j + 1] = if x == y {
            d + 1
          } else {
            row[j + 1].max(row[j])
          };
          d = up;
        }
      }
      row[b.len()]
    };
    for (symbols, bw, center) in [(32u32, 1000.0, 1500.0), (8, 250.0, 750.0)] {
      let clean = encode_audio(&text, symbols, bw, center, 8000).unwrap();
      for ppm in [300.0, -300.0, 600.0] {
        let drifted = resample_ppm(&clean, ppm);
        let noise = white_noise(7 + (ppm as i64 + 1000) as u64, drifted.len());
        let noisy: Vec<f32> = drifted
          .iter()
          .zip(&noise)
          .map(|(a, n)| a + 1.0 * n)
          .collect();
        let plain = decode_audio(&noisy, 8000, symbols, bw, center).map_or(0, |s| lcs(&text, &s));
        let (found, chosen) = decode_audio_drift_search(&noisy, 8000, symbols, bw, center)
          .unwrap_or_else(|| panic!("{symbols}/{bw} {ppm} ppm：扫描后仍无结果"));
        let got = lcs(&text, &found);
        assert!(
          got >= text.len() - 4,
          "{symbols}/{bw} {ppm} ppm：扫描后仅还原 {got}/{}",
          text.len()
        );
        assert!(
          got >= plain,
          "{symbols}/{bw} {ppm} ppm：扫描不应比不扫更差（{got} < {plain}）"
        );
        assert!(chosen.abs() <= 800.0);
      }
    }
  }

  #[test]
  fn drift_search_leaves_a_stable_recording_at_zero_ppm() {
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 ".repeat(8);
    let clean = encode_audio(&text, 32, 1000.0, 1500.0, 8000).unwrap();
    let (found, ppm) = decode_audio_drift_search(&clean, 8000, 32, 1000.0, 1500.0).expect("应解出");
    assert_eq!(ppm, 0.0, "不漂移的录音不该被「修正」");
    assert_eq!(found, text);
  }

  #[test]
  fn drift_search_does_not_turn_silence_or_noise_into_text() {
    assert!(
      decode_audio_drift_search(&vec![0.0f32; 8000 * 10], 8000, 32, 1000.0, 1500.0).is_none()
    );
    let noise = white_noise(99, 8000 * 20);
    for (symbols, bw, center) in MODES {
      assert!(
        decode_audio_drift_search(&noise, 8000, symbols, bw, center).is_none(),
        "{symbols}/{bw}：白噪声被扫描「解」出了文本"
      );
    }
  }

  #[test]
  fn white_noise_is_almost_never_decoded_in_any_submode() {
    // 修复前：窄带子模式（8/250、4/125）对白噪声几乎 100% 给出十来个乱码字符。
    // 门限：连续锁定 ≥ 3 块（见 `MIN_RUN_WITHOUT_PREAMBLE`）。
    for (symbols, bw, center) in MODES {
      let hits = (0..60u64)
        .filter(|&k| {
          let s = white_noise(0x1000 + k * 7919, 8000 * 20);
          decode_audio(&s, 8000, symbols, bw, center).is_some()
        })
        .count();
      assert!(
        hits <= 1,
        "{symbols}/{bw}：60 次白噪声里 {hits} 次给出了文本"
      );
    }
  }

  #[test]
  fn real_signal_survives_noise_in_every_submode() {
    // 收紧噪声门限不能误杀带噪的真信号。
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG";
    for (symbols, bw, center) in MODES {
      let clean = encode_audio(text, symbols, bw, center, 8000).unwrap();
      for (k, amp) in [0.0f32, 0.5, 1.0].into_iter().enumerate() {
        let noise = white_noise(77 + k as u64, clean.len());
        let noisy: Vec<f32> = clean.iter().zip(&noise).map(|(a, n)| a + amp * n).collect();
        let got = decode_audio(&noisy, 8000, symbols, bw, center)
          .unwrap_or_else(|| panic!("{symbols}/{bw} 噪声幅度 {amp}：真信号被拒收"));
        assert!(
          got.contains("QUICK") && got.contains("LAZY"),
          "{symbols}/{bw} {amp}: {got}"
        );
      }
    }
  }

  #[test]
  fn a_long_message_without_preamble_is_still_accepted() {
    // 没有前导（录音从中途开始）时靠「连续锁定」放行；长报文远超 3 块。
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG";
    for (symbols, bw, center) in MODES {
      let full = encode_audio(text, symbols, bw, center, 8000).unwrap();
      let wlen = (8000.0 / (bw / f64::from(symbols))).ceil() as usize;
      let cut = &full[BLOCK_SYMBOLS * wlen..];
      let got = decode_audio(cut, 8000, symbols, bw, center)
        .unwrap_or_else(|| panic!("{symbols}/{bw}：无前导的长报文被拒收"));
      assert!(got.contains("LAZY"), "{symbols}/{bw}: {got}");
    }
  }

  #[test]
  fn silence_yields_nothing() {
    // R2-08 / R2-26：静音必须是 `None`，而不是 `Some("")`。
    let silence = vec![0.0f32; 8000 * 10];
    assert!(decode_audio(&silence, 8000, 32, 1000.0, 1500.0).is_none());
    assert!(decode_audio(&[], 8000, 32, 1000.0, 1500.0).is_none());
  }

  #[test]
  fn noise_does_not_panic_and_reports_what_it_decoded() {
    // 噪声下 WHT 门限只是概率性的（见 docs/decode-worker-protocol.md「门限是概率性的」）：
    // 这里只钉死「不 panic、结果只含可打印字符或为 None」，不宣称噪声必为 None。
    for seed0 in [0x1234_5678u32, 0x9E37_79B9, 0x5BD1_E995, 0x85EB_CA6B] {
      let mut seed = seed0;
      let samples: Vec<f32> = (0..8000 * 12)
        .map(|_| {
          seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
          ((seed >> 8) as f32 / 8_388_608.0) - 1.0
        })
        .collect();
      if let Some(t) = decode_audio(&samples, 8000, 32, 1000.0, 1500.0) {
        assert!(!t.is_empty(), "噪声不得返回 Some(\"\")，seed {seed0:#x}");
      }
    }
  }

  #[test]
  fn public_api_rejects_hostile_parameters() {
    // R2-07：symbols = 4e9 曾能通过校验并让每窗口跑 40 亿次 Goertzel。
    let samples = vec![0.5f32; 8000];
    assert!(demodulate(&samples, 8000, 4_000_000_000, 4e9 * 31.25, 1500.0).is_empty());
    assert!(demodulate(&samples, 8000, 5, 156.25, 1500.0).is_empty());
    // bandwidth 极小 → wlen 饱和成 usize::MAX，`i + wlen` 曾回绕。
    assert!(demodulate(&samples, 8000, 32, 1e-300, 1500.0).is_empty());
    // R2-09：巨大采样率不得分配数十 GB。
    assert!(encode_audio("A", 32, 1000.0, 1500.0, u32::MAX).is_none());
  }

  #[test]
  fn round_trips_default_submode() {
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 12345";
    let audio = encode_audio(text, 32, 1000.0, 1500.0, 8000).unwrap();
    let decoded = decode_audio(&audio, 8000, 32, 1000.0, 1500.0).unwrap();
    assert_eq!(decoded, text, "32/1000 往返");
  }

  #[test]
  fn round_trips_8_250() {
    let text = "CQ CQ DE BG1AAA 599";
    let audio = encode_audio(text, 8, 250.0, 750.0, 8000).unwrap();
    let decoded = decode_audio(&audio, 8000, 8, 250.0, 750.0).unwrap();
    assert_eq!(decoded, text, "8/250 往返");
  }

  #[test]
  fn round_trips_16_500() {
    let text = "OLIVIA MODE TEST";
    let audio = encode_audio(text, 16, 500.0, 1000.0, 8000).unwrap();
    let decoded = decode_audio(&audio, 8000, 16, 500.0, 1000.0).unwrap();
    assert_eq!(decoded, text, "16/500 往返");
  }

  #[test]
  fn round_trips_4_125() {
    let text = "MINI";
    let audio = encode_audio(text, 4, 125.0, 750.0, 8000).unwrap();
    let decoded = decode_audio(&audio, 8000, 4, 125.0, 750.0).unwrap();
    assert_eq!(decoded, text, "4/125 往返");
  }

  #[test]
  fn decode_handles_arbitrary_leading_offset() {
    let text = "OFFSET TEST 123";
    let audio = encode_audio(text, 32, 1000.0, 1500.0, 8000).unwrap();
    let mut padded = vec![0.0f32; 37];
    padded.extend_from_slice(&audio);
    let decoded = decode_audio(&padded, 8000, 32, 1000.0, 1500.0).unwrap();
    assert_eq!(decoded, text, "前导静音后仍应解出原文");
  }

  #[test]
  fn walsh_and_key_round_trip() {
    // 直接验证编码块的符号能被 decode_block 解回原字符。
    let chars = b"ABC\0\0".to_vec();
    let syms = encode_block(&chars, 5);
    let (back, _, doubt) = decode_block(&syms, 5);
    assert_eq!(doubt, 0);
    assert_eq!(&back[..3], b"ABC");
  }

  #[test]
  fn damaged_first_block_is_not_skipped_for_a_clean_later_block() {
    let mut first = encode_block(b"FIRST", 5);
    first[0] ^= 1;
    let last = encode_block(b"LAST!", 5);
    let symbols: Vec<u32> = first.into_iter().chain(last).collect();
    assert_eq!(decode_symbols(&symbols, 5), "FIRSTLAST!");
  }

  #[test]
  fn locked_stream_keeps_all_blocks_in_order() {
    let block = encode_block(b"ABCDE", 5);
    let symbols: Vec<u32> = (0..1024).flat_map(|_| block).collect();
    assert_eq!(decode_symbols(&symbols, 5), "ABCDE".repeat(1024));
    assert!(decode_symbols(&symbols, 0).is_empty());
    assert!(decode_symbols(&symbols, 32).is_empty());
  }

  #[test]
  fn encode_rejects_absurd_bandwidths() {
    // fsep 极小（bandwidth = 0.001）时 wlen 会大到给 preamble 申请几十 GB；
    // fsep 极大（4/500@750 的 125 Hz）时长度下溢。两个方向都要拒绝。
    assert!(encode_audio("A", 32, 0.001, 1500.0, 8000).is_none());
    assert!(encode_audio("A", 4, 500.0, 750.0, 8000).is_none());
    // 标准子模式（fsep = 31.25 Hz）不受影响。
    assert!(encode_audio("A", 32, 1000.0, 1500.0, 8000).is_some());
  }

  #[test]
  fn parse_submode_accepts_standard_forms() {
    assert_eq!(parse_submode("32/1000@1500"), Some((32, 1000.0, 1500.0)));
    assert_eq!(parse_submode("8/250@750"), Some((8, 250.0, 750.0)));
    // 语法解析不校验音调数（合法性在 bits_per_symbol），格式不对才返回 None。
    assert!(parse_submode("32/1000").is_none());
    assert!(bits_per_symbol(9).is_none());
  }
}
