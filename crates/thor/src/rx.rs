//! THOR 收端：音频 → 文本。判决部分对应 fldigi `thor::decodesymbol`（硬位路径）/ `decodePairs`；
//! 前端（中心频率、符号对齐、质量门限）与 MFSK 共用 [`ham_web_mfsk::front`]。

use ham_web_mfsk::fec::{Interleaver, PUNCTURE, Viterbi};
use ham_web_mfsk::front::{
  Grid, detect_base as detect_base_grid, find_offset, gate, peak_ratios, polish_base, symbol_bins,
};
use ham_web_mfsk::lookup::{render, varidec};

use crate::varicode::SECONDARY;
use crate::{Mode, NUMTONES};

/// 主 / 二次信道的分界：THOR 二次信道的码字都不小于它（fldigi `thorvaridec`）。
const SECONDARY_MIN: u32 = 0xB80;

/// 「最强音调 / 平均幅度」低于它就当作没有清晰音调（宽带脉冲或噪声），整个 nibble 删除（fldigi
/// `staticburst`）。
const STATIC_BURST_RATIO: f64 = 1.2;

/// 区间内音调差不合法（0 / 1 / −17：IFK+ 不会出现）的 nibble 占比上限。
///
/// IFK+ 的增量恒为 `2 + nibble` ∈ 2..=17，同一音调连续出现或只差 1 只可能来自噪声里的随机命中
/// （约 9%）；真实信号上它接近 0。峰均比门限已经挡掉了纯噪声，这里再挡一道「有能量但不是 IFK+」。
const MAX_INVALID_PERCENT: usize = 10;

/// 解码结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
  /// 主信道正文（`\r` 换成换行，其余控制字符去掉）。
  pub text: String,
  /// 二次信道（ID 信标）文本，可能为空。
  pub id: String,
  /// 使用的信号中心频率（Hz）：调用方没给就是自动检测的结果。
  pub center_hz: f64,
  /// 参与解码的符号数。
  pub symbols: usize,
}

fn grid_for(rate: u32, mode: &Mode) -> Option<Grid> {
  Grid::new(rate, mode.baud(), mode.spacing(), NUMTONES)
}

/// 估计信号**最低音调**的频率（Hz）：见 [`ham_web_mfsk::front::detect_base`]。
#[must_use]
pub fn detect_base(samples: &[f32], rate: u32, mode: &Mode) -> Option<f64> {
  detect_base_grid(samples, &grid_for(rate, mode)?)
}

/// fldigi `thorvaridec`：码字 → 字符；二次信道字符返回 `0x100 | 字符`。
fn thor_varidec(code: u32) -> Option<u16> {
  if code < SECONDARY_MIN {
    return varidec(code);
  }
  SECONDARY
    .iter()
    .position(|s| u32::from_str_radix(s, 2) == Ok(code))
    .map(|i| 0x100 | (u16::from(b' ') + i as u16))
}

/// 音调判决 + IFK+ 差分 + 解交织 + 译码 + 切字符。
struct Receiver {
  depth: usize,
  traceback: usize,
  inlv: Interleaver,
  vit: Viterbi,
  pair: [u8; 2],
  first_of_pair: bool,
  datashreg: u32,
  prev: Option<usize>,
  chars: Vec<u16>,
  /// 参与差分的 nibble 数与其中音调差不合法的个数。
  events: usize,
  invalid: usize,
}

impl Receiver {
  fn new(mode: &Mode) -> Self {
    Self {
      depth: mode.depth,
      traceback: mode.traceback(),
      inlv: Interleaver::new(4, mode.depth, false),
      vit: Viterbi::new(mode.code(), mode.traceback()),
      pair: [0; 2],
      first_of_pair: false,
      datashreg: 1,
      prev: None,
      chars: Vec::new(),
      events: 0,
      invalid: 0,
    }
  }

  /// 一个符号的各音调幅度 → 与上一个符号的音调差 → 4 个码位软值。
  fn feed(&mut self, bins: &[f64]) {
    let n = bins.len() as f64;
    let avg = (bins.iter().sum::<f64>() / n).max(1e-20);
    let mut best = (0.0f64, 0usize);
    for (i, &x) in bins.iter().enumerate() {
      if x > best.0 {
        best = (x, i);
      }
    }
    let burst = best.0 / avg < STATIC_BURST_RATIO;
    let Some(prev) = self.prev.replace(best.1) else {
      return;
    };
    let d = best.1 as i32 - prev as i32;
    self.events += 1;
    // 合法的音调差是 2..=17 或 −16..=−1；0 / 1 / −17 不是 IFK+ 能产生的。
    let valid = !matches!(d, 0 | 1 | -17);
    if !valid {
      self.invalid += 1;
    }
    let mut syms = [PUNCTURE; 4];
    if valid && !burst {
      let c = (d - 2).rem_euclid(NUMTONES as i32) as u8;
      for (i, s) in syms.iter_mut().enumerate() {
        *s = if (c >> (3 - i)) & 1 == 1 { 255 } else { 0 };
      }
    }
    self.inlv.symbols(&mut syms);
    for s in syms {
      self.decode_pair(s);
    }
  }

  /// 信号结束后把管线里压着的字符推出来：喂若干个「全是删除位」的 nibble。
  ///
  /// 反向交织器（`3 × 深度` 个 nibble）与 Viterbi 回溯（每个 nibble 译出 2 位）合起来有一段固定延迟，
  /// 信号最后那几个字符要等这么多个 nibble 才流得出来 —— THOR 发端只补 4 个空闲符的冲刷，远不够填满
  /// 这段延迟，fldigi 靠接收机收完信号后继续运行（噪声把它们推出来）。离线文件没有「后面」，所以在这里补。
  /// 删除位的度量增量为 0，不会改变已有路径的取舍，只是让回溯窗口往前走。
  fn flush(&mut self) {
    for _ in 0..3 * self.depth + self.traceback.div_ceil(2) {
      let mut syms = [PUNCTURE; 4];
      self.inlv.symbols(&mut syms);
      for s in syms {
        self.decode_pair(s);
      }
    }
  }

  /// fldigi `decodePairs`：软符号两两配对进 Viterbi，输出位按 `001` 切字符。
  fn decode_pair(&mut self, symbol: u8) {
    self.pair = [self.pair[1], symbol];
    self.first_of_pair = !self.first_of_pair;
    if self.first_of_pair {
      return;
    }
    let (bit, _) = self.vit.decode(self.pair);
    self.datashreg = (self.datashreg << 1) | u32::from(bit);
    if self.datashreg & 7 == 1 {
      if let Some(c) = thor_varidec(self.datashreg >> 1) {
        self.chars.push(c);
      }
      self.datashreg = 1;
    }
  }
}

/// 完整解码：音频 → 文本。
///
/// - `center`：信号中心频率（Hz）；`None` 时自动估计（[`detect_base`]）；
/// - `reverse`：边带反转（LSB 录音）。
///
/// 噪声与静音返回 `None`：峰均比划不出信号区间、区间的平均峰均比不够、或区间里不合法的音调差
/// 太多（见 [`ham_web_mfsk::front::gate`] 与 [`MAX_INVALID_PERCENT`]）。
#[must_use]
pub fn decode_audio(
  samples: &[f32],
  rate: u32,
  mode: &Mode,
  center: Option<f64>,
  reverse: bool,
) -> Option<Decoded> {
  let grid = grid_for(rate, mode)?;
  let (base, auto) = match center {
    Some(c) if c.is_finite() => (c + mode.tone_offset(0), false),
    Some(_) => return None,
    None => (detect_base_grid(samples, &grid)?, true),
  };
  if !grid.fits(base) {
    return None;
  }
  let offset = find_offset(samples, &grid, base);
  // 自动检测的中心频率在符号对齐之后再核对一遍；用户给定的不动。
  let base = if auto {
    polish_base(samples, &grid, base, offset)
  } else {
    base
  };
  if !grid.fits(base) {
    return None;
  }
  let nt = grid.ntones;
  let bins = symbol_bins(samples, &grid, base, offset);
  let (first, last) = gate(&peak_ratios(&bins, nt))?;
  let span = last - first + 1;

  let mut rx = Receiver::new(mode);
  let mut tmp = vec![0.0f64; nt];
  for chunk in bins.chunks_exact(nt).skip(first).take(span) {
    if reverse {
      for (t, &v) in tmp.iter_mut().zip(chunk.iter().rev()) {
        *t = v;
      }
      rx.feed(&tmp);
    } else {
      rx.feed(chunk);
    }
  }
  rx.flush();
  if rx.events == 0 || rx.invalid * 100 > rx.events * MAX_INVALID_PERCENT {
    return None;
  }
  let primary: Vec<u16> = rx.chars.iter().copied().filter(|&c| c < 0x100).collect();
  let secondary: Vec<u16> = rx
    .chars
    .iter()
    .filter(|&&c| c >= 0x100)
    .map(|&c| c & 0xFF)
    .collect();
  Some(Decoded {
    text: render(&primary),
    id: render(&secondary),
    center_hz: base - mode.tone_offset(0),
    symbols: span,
  })
}
