//! 真实 Meteor-M2 录音（72k QPSK，任意采样率）的接收前端：
//! RRC 匹配滤波 → Gardner 定时恢复 → Costas 载波恢复 → 逐帧解相位模糊 → Viterbi → 帧。
//!
//! 与 [`crate::rrc`]（288 kHz、整数 4 倍采样、无频偏的 OQPSK，对齐 lrpt-encoder 的合成信号）
//! 互补：真实过境录音的采样率由录音软件决定（SDR# 常见 187.5 / 250 kHz），每符号点数不是
//! 整数；还有多普勒与本振误差带来的 ±几 kHz 频偏。所以这里不靠「整数相位搜索」，而是
//! 环路跟踪：
//!
//! - **定时**：Gardner 误差检测器（每符号一个判决点 + 半符号中点，线性插值），只用比例项；
//! - **载波**：二阶 Costas 环（判决引导 QPSK 误差 `sgn(I)·Q − sgn(Q)·I`），频率积分初值 0，
//!   能拉进 ±1.5 kHz 级的偏移（测试录音实测 +1.2 → −0.5 kHz 的多普勒漂移全程跟住）；
//! - **相位模糊**：QPSK 的 Costas 环有 4 重相位模糊，录音软件还可能把频谱镜像（I/Q 对调），
//!   一共 8 种假设；环路中途周跳时正确的假设会**变**。所以不在全流上选一次，而是用
//!   卷积编码后的 ASM 段在 8 种假设上逐符号相关，**每一帧各自**决定用哪一种。
//!
//! 链路参数按 Meteor-M2（2014 年发射）的 72k LRPT：QPSK、RRC 滚降 0.6、**无差分编码**
//! （ASM 相关能在不做 NRZ-M 的位流上直接命中，就是证据；SatDump 的 `meteor_m2_lrpt` 管线
//! 同样 `diff_decode = false`）。M2-3 / M2-4 的 OQPSK 不走这里。
//!
//! 已在一段公开的真实录音上验证（SigIDWiki LRPT 词条附带的 SDR# 基带，137.9 MHz、
//! 187.5 kHz、105 s）：见 `tests/real_iq.rs`（录音不进仓库，用环境变量给路径）。

use crate::DATA_LEN;
use crate::cadu::{ASM, FRAME_LEN};
use crate::prand::PRAND;
use crate::viterbi::{BRANCH_TABLE, TrellisBuf, viterbi_decode_window_reuse};

/// Meteor-M2 LRPT 符号率。
pub const SYMBOL_RATE: f64 = 72_000.0;
/// RRC 滚降。
pub const ROLLOFF: f64 = 0.6;
/// RRC 跨度（符号）。
const SPAN_SYMBOLS: f64 = 16.0;
/// 定时环 / 载波环的归一化环路带宽（每符号）与阻尼。
const TIMING_BW: f64 = 0.002;
const CARRIER_BW: f64 = 0.01;
const ZETA: f64 = 0.707;
/// ASM 相关允许的错位数（编码后 ASM 的 26 个符号 = 52 位里）。
const ASM_TOLERANCE: u32 = 3;
/// 每帧前后给 Viterbi 留的收敛余量（符号 = 数据位）。
const MARGIN: usize = 96;
/// 帧长（符号）：一帧 8192 数据位，码率 1/2、每符号 2 个编码位 → 8192 个符号。
const FRAME_SYMBOLS: usize = FRAME_LEN * 8;

/// 复数（f64，只用到这里需要的运算）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct C(f64, f64);

impl C {
  fn mul(self, o: Self) -> Self {
    Self(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
  }
  fn conj(self) -> Self {
    Self(self.0, -self.1)
  }
  fn scale(self, k: f64) -> Self {
    Self(self.0 * k, self.1 * k)
  }
  fn add(self, o: Self) -> Self {
    Self(self.0 + o.0, self.1 + o.1)
  }
  fn sub(self, o: Self) -> Self {
    Self(self.0 - o.0, self.1 - o.1)
  }
  fn expj(phi: f64) -> Self {
    Self(phi.cos(), phi.sin())
  }
}

/// 按需计算的匹配滤波输出：只缓存环路还会用到的一小段（滑动窗口），内存与录音长度无关。
struct MatchedFilter<'a> {
  iq: &'a [f32],
  taps: &'a [f64],
  norm: f64,
  /// 缓存里第一个点的下标。
  first: usize,
  buf: std::collections::VecDeque<C>,
}

impl<'a> MatchedFilter<'a> {
  fn new(iq: &'a [f32], taps: &'a [f64], norm: f64) -> Self {
    Self {
      iq,
      taps,
      norm,
      first: 0,
      buf: std::collections::VecDeque::new(),
    }
  }

  /// 第 `i` 个输出点（交织 I/Q 里从第 `i` 个复数样本起的 `taps.len()` 点卷积，再乘归一化系数）。
  fn compute(&self, i: usize) -> C {
    let mut acc = C::default();
    for (k, &h) in self.taps.iter().enumerate() {
      let j = 2 * (i + k);
      acc = acc.add(C(f64::from(self.iq[j]), f64::from(self.iq[j + 1])).scale(h));
    }
    acc.scale(self.norm)
  }

  fn get(&mut self, i: usize) -> C {
    // 定时环偶尔会回看到已释放的点（只会少量，不值得为它保留更长的窗口）：直接重算。
    if i < self.first {
      return self.compute(i);
    }
    while self.first + self.buf.len() <= i {
      let next = self.compute(self.first + self.buf.len());
      self.buf.push_back(next);
    }
    self.buf[i - self.first]
  }

  /// 丢掉下标小于 `below` 的缓存点。
  fn release(&mut self, below: usize) {
    while self.first < below && self.buf.pop_front().is_some() {
      self.first += 1;
    }
  }
}

/// 匹配滤波抽头数上限。真实录音的每符号点数在 2.6–4（n ≈ 41–65）；到 1024 抽头
/// 已对应 4.6 MHz 级采样率，再往上要么是构造出来的输入（`usize` 饱和会直接 OOM），
/// 要么卷积量本身已不可能在 worker 内完成 —— 都直接拒绝。
const MAX_TAPS: usize = 1024;

/// 归一化 RRC 系数（每符号 `sps` 个点，可为小数），抽头数取奇数。
fn rrc_taps(sps: f64, alpha: f64) -> Vec<f64> {
  let n = ((SPAN_SYMBOLS * sps) as usize) | 1;
  let half = (n / 2) as f64;
  let pi = std::f64::consts::PI;
  let mut h: Vec<f64> = (0..n)
    .map(|i| {
      let t = (i as f64 - half) / sps;
      if t.abs() < 1e-9 {
        1.0 - alpha + 4.0 * alpha / pi
      } else if (t.abs() - 1.0 / (4.0 * alpha)).abs() < 1e-9 {
        alpha / 2f64.sqrt()
          * ((1.0 + 2.0 / pi) * (pi / (4.0 * alpha)).sin()
            + (1.0 - 2.0 / pi) * (pi / (4.0 * alpha)).cos())
      } else {
        ((pi * t * (1.0 - alpha)).sin() + 4.0 * alpha * t * (pi * t * (1.0 + alpha)).cos())
          / (pi * t * (1.0 - (4.0 * alpha * t).powi(2)))
      }
    })
    .collect();
  let e = h.iter().map(|v| v * v).sum::<f64>().sqrt();
  h.iter_mut().for_each(|v| *v /= e);
  h
}

/// 二阶环路的比例 / 积分增益（归一化带宽 `bw`，阻尼 [`ZETA`]）。
fn loop_gains(bw: f64) -> (f64, f64) {
  let th = bw / (ZETA + 1.0 / (4.0 * ZETA));
  let d = 1.0 + 2.0 * ZETA * th + th * th;
  (4.0 * ZETA * th / d, 4.0 * th * th / d)
}

/// 交织 I/Q（f32）→ 每符号一个复数判决点（已去载波、未解相位模糊）。
///
/// 输入功率先归一化，输出星座半径约为 1。`sample_rate` 不足每符号 2 点时返回空。
#[must_use]
pub fn demodulate(iq: &[f32], sample_rate: f64) -> Vec<(f32, f32)> {
  let sps = sample_rate / SYMBOL_RATE;
  if !sps.is_finite() || sps < 2.0 {
    return Vec::new();
  }
  let n = iq.len() / 2;
  // 先算抽头数再过短输入 / 上限两道守卫，再分配：`as usize` 对极端值会饱和，
  // 先分配会让 wasm 直接 OOM（见 `MAX_TAPS`）。
  let taps_len = ((SPAN_SYMBOLS * sps) as usize) | 1;
  if taps_len > MAX_TAPS || n < taps_len * 4 {
    return Vec::new();
  }
  let taps = rrc_taps(sps, ROLLOFF);
  let power = iq.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / n as f64;
  if power <= 0.0 || !power.is_finite() {
    return Vec::new();
  }
  let norm = 1.0 / power.sqrt();
  // 匹配滤波按需计算（输出与输入对齐的 "same" 卷积，两端各缺半个滤波器长度的点直接不算）。
  // 不整段预先算好：105 s、187.5 kHz 的录音有近两千万个点，整段存下来要 300 MB 以上，
  // 而环路每一步只用得到 `t` 附近的几个点。
  let m = taps.len();
  let x_len = n - m + 1;
  let mut x = MatchedFilter::new(iq, &taps, norm);
  let at = |x: &mut MatchedFilter, t: f64| -> C {
    let i = t as usize;
    let f = t - i as f64;
    x.get(i).scale(1.0 - f).add(x.get(i + 1).scale(f))
  };

  let (kp_t, _) = loop_gains(TIMING_BW);
  let (kp_c, ki_c) = loop_gains(CARRIER_BW);
  let (mut t, mut phase, mut freq) = (sps.ceil(), 0.0f64, 0.0f64);
  let mut prev = C::default();
  let mut out = Vec::with_capacity((x_len as f64 / sps) as usize + 1);
  while t + 2.0 < x_len as f64 {
    // 中点要回看半个符号，更早的点不会再被用到。
    x.release(((t - sps / 2.0).max(0.0) as usize).saturating_sub(2));
    let r = C::expj(-phase);
    let cur = at(&mut x, t).mul(r);
    // 中点在半个符号之前：那时的载波相位还没走完这一步的 `freq`。
    let mid = at(&mut x, t - sps / 2.0).mul(r).mul(C::expj(freq / 2.0));
    let e_t = prev.sub(cur).mul(mid.conj()).0;
    t += sps * (1.0 + kp_t * e_t);
    let e_c = cur.0.signum() * cur.1 - cur.1.signum() * cur.0;
    freq += ki_c * e_c;
    phase += freq + kp_c * e_c;
    // 相位取模，长录音里也不损失精度。
    phase = phase.rem_euclid(std::f64::consts::TAU);
    out.push((cur.0 as f32, cur.1 as f32));
    prev = cur;
  }
  out
}

/// 8 种相位 / 镜像假设之一作用到一个符号上，返回两个硬判决编码位（G1, G2）。
fn bits_of(sym: (f32, f32), variant: u8) -> (u8, u8) {
  let (mut i, mut q) = sym;
  for _ in 0..(variant & 3) {
    (i, q) = (-q, i); // ×j
  }
  if variant & 4 != 0 {
    q = -q; // 镜像（共轭）
  }
  (u8::from(i < 0.0), u8::from(q < 0.0))
}

/// ASM 第 6 位起的编码位（前 6 位的编码输出依赖 ASM 之前的数据，不确定）：26 个符号。
fn encoded_asm() -> [(u8, u8); 26] {
  let mut state = 0usize;
  let mut out = [(0u8, 0u8); 26];
  for k in 0..32 {
    let bit = (ASM[k / 8] >> (7 - k % 8)) & 1;
    let (next, b0, b1) = BRANCH_TABLE[state][usize::from(bit)];
    if k >= 6 {
      out[k - 6] = (b0, b1);
    }
    state = next;
  }
  out
}

/// 符号流 → 帧（已去随机化的 1020 字节，与 [`crate::decode_stream288`] 同口径）。
///
/// 返回 `(帧, 每帧采用的假设)`；后者供诊断（周跳时会变）。
#[must_use]
pub fn frames_from_symbols(syms: &[(f32, f32)]) -> (Vec<[u8; DATA_LEN]>, Vec<u8>) {
  let pattern = encoded_asm();
  let mut frames = Vec::new();
  let mut variants = Vec::new();
  // 跨帧复用译码缓冲与编码位数组：每帧窗口等长，逐帧新分配会白白写 ~1.1 MB×帧数。
  let mut vbuf = TrellisBuf::default();
  let mut coded: Vec<u8> = Vec::new();
  let mut p = MARGIN + 6;
  while p + pattern.len() + FRAME_SYMBOLS + MARGIN < syms.len() {
    let hit = (0u8..8).find(|&v| {
      let mut bad = 0u32;
      for (k, &(a, b)) in pattern.iter().enumerate() {
        let (x, y) = bits_of(syms[p + k], v);
        bad += u32::from(x != a) + u32::from(y != b);
        if bad > ASM_TOLERANCE {
          return false;
        }
      }
      true
    });
    let Some(v) = hit else {
      p += 1;
      continue;
    };
    // ASM 第 0 位所在的符号。
    let start = p - 6;
    let window = &syms[start - MARGIN..start + FRAME_SYMBOLS + MARGIN];
    coded.clear();
    coded.extend(window.iter().flat_map(|&s| {
      let (a, b) = bits_of(s, v);
      [a, b]
    }));
    let bits = viterbi_decode_window_reuse(&coded, &mut vbuf);
    let mut frame = [0u8; FRAME_LEN];
    for (i, byte) in frame.iter_mut().enumerate() {
      *byte = bits[MARGIN + i * 8..MARGIN + i * 8 + 8]
        .iter()
        .fold(0u8, |acc, &b| (acc << 1) | b);
    }
    let asm_bad: u32 = frame[..4]
      .iter()
      .zip(ASM)
      .map(|(a, b)| (a ^ b).count_ones())
      .sum();
    if asm_bad <= 2 {
      let mut data = [0u8; DATA_LEN];
      for (i, d) in data.iter_mut().enumerate() {
        *d = frame[ASM.len() + i] ^ PRAND[i % PRAND.len()];
      }
      frames.push(data);
      variants.push(v);
      // 下一帧的 ASM 至少在一帧之后：跳过帧体，留几个符号的定时抖动余量。
      p += FRAME_SYMBOLS - 16;
    } else {
      p += 1;
    }
  }
  (frames, variants)
}

/// 完整前端：交织 I/Q（任意采样率，≥ 144 kHz）→ 帧。
#[must_use]
pub fn decode_stream_qpsk(iq: &[f32], sample_rate: f64) -> Vec<[u8; DATA_LEN]> {
  frames_from_symbols(&demodulate(iq, sample_rate)).0
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::viterbi::conv_encode_continuous;

  /// 合成一段 QPSK：若干帧（ASM + 随机数据）→ 连续卷积 → 符号 → RRC 成形 →
  /// 指定采样率、频偏、相位与镜像。
  fn synth(
    frames: usize,
    rate: f64,
    offset_hz: f64,
    phase0: f64,
    mirror: bool,
  ) -> (Vec<f32>, Vec<[u8; DATA_LEN]>) {
    let mut seed = 0x2468_ACE1u32;
    let mut bits = Vec::new();
    let mut truth = Vec::new();
    for _ in 0..frames {
      let mut frame = [0u8; FRAME_LEN];
      frame[..4].copy_from_slice(&ASM);
      for b in &mut frame[4..] {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        *b = (seed >> 16) as u8;
      }
      let mut data = [0u8; DATA_LEN];
      for (i, d) in data.iter_mut().enumerate() {
        *d = frame[4 + i] ^ PRAND[i % PRAND.len()];
      }
      truth.push(data);
      bits.extend(
        frame
          .iter()
          .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1)),
      );
    }
    let mut state = 0u8;
    let coded = conv_encode_continuous(&bits, &mut state);
    let syms: Vec<C> = coded
      .chunks(2)
      .map(|p| C(1.0 - 2.0 * f64::from(p[0]), 1.0 - 2.0 * f64::from(p[1])))
      .collect();
    let sps = rate / SYMBOL_RATE;
    let taps = rrc_taps(sps, ROLLOFF);
    let n = (syms.len() as f64 * sps) as usize + taps.len();
    let half = (taps.len() / 2) as f64;
    let mut out = Vec::with_capacity(2 * n);
    for i in 0..n {
      // 成形：对每个采样点把附近符号的脉冲叠加起来（按连续时间求值，任意 sps 都成立）。
      let t = i as f64 / sps;
      let mut acc = C::default();
      let k0 = (t - half / sps).floor().max(0.0) as usize;
      let k1 = ((t + half / sps).ceil() as usize).min(syms.len());
      for (k, s) in syms.iter().enumerate().take(k1).skip(k0) {
        let d = (t - k as f64) * sps + half;
        let j = d.round();
        if (0.0..taps.len() as f64).contains(&j) && (d - j).abs() < 1e-9 {
          acc = acc.add(s.scale(taps[j as usize]));
        }
      }
      let rot = C::expj(phase0 + std::f64::consts::TAU * offset_hz * i as f64 / rate);
      let mut v = acc.mul(rot);
      if mirror {
        v = v.conj();
      }
      out.push(v.0 as f32);
      out.push(v.1 as f32);
    }
    (out, truth)
  }

  #[test]
  fn recovers_frames_through_offset_rotation_and_mirror() {
    // 每符号整数 4 点（成形在整数网格上精确），频偏 +800 Hz、任意初相、镜像。
    let (iq, truth) = synth(6, 288_000.0, 800.0, 1.0, true);
    let (frames, variants) = frames_from_symbols(&demodulate(&iq, 288_000.0));
    // 首帧前要留 MARGIN + 环路收敛，首尾各丢一帧是正常的。
    assert!(frames.len() >= 4, "只解出 {} 帧", frames.len());
    for f in &frames {
      assert!(truth.contains(f), "解出的帧必须是真值之一（逐字节）");
    }
    assert!(
      variants.iter().all(|&v| v & 4 != 0),
      "镜像应被识别：{variants:?}"
    );
  }

  #[test]
  fn rejects_noise_and_too_low_rates() {
    let mut seed = 7u32;
    let noise: Vec<f32> = (0..400_000)
      .map(|_| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        (f64::from(seed >> 8) / f64::from(1u32 << 24) - 0.5) as f32
      })
      .collect();
    assert!(decode_stream_qpsk(&noise, 288_000.0).is_empty());
    assert!(demodulate(&noise, 100_000.0).is_empty(), "每符号不足 2 点");
    assert!(demodulate(&[], 288_000.0).is_empty());
    // 极端采样率：抽头数远超上限，必须在分配前拒绝（曾会 usize 饱和后 OOM）。
    assert!(demodulate(&noise, 1e18).is_empty());
    assert!(demodulate(&noise, f64::NAN).is_empty());
  }

  #[test]
  fn encoded_asm_matches_the_convolutional_encoder() {
    // 26 个确定符号必须与编码器对 ASM 的输出（第 6 位起）逐位相同，与 ASM 之前的数据无关。
    for prefix in [0u8, 0xFF, 0x5A] {
      let bits: Vec<u8> = [prefix]
        .iter()
        .chain(ASM.iter())
        .flat_map(|&b| (0..8).map(move |i| (b >> (7 - i)) & 1))
        .collect();
      let mut state = 0u8;
      let coded = conv_encode_continuous(&bits, &mut state);
      let tail: Vec<(u8, u8)> = coded[2 * 14..].chunks(2).map(|p| (p[0], p[1])).collect();
      assert_eq!(tail, encoded_asm().to_vec(), "前缀 {prefix:#04x}");
    }
  }
}
