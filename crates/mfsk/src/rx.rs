//! MFSK 收端：音频 → 文本。判决部分逐步对应 fldigi `mfsk::harddecode` / `softdecode` /
//! `decodesymbol` / `recvbit`；前端（中心频率、符号对齐、质量门限）是离线文件专用的重写。

use crate::fec::{Code, Interleaver, PUNCTURE, Viterbi};
use crate::front::{
  Grid, detect_base as detect_base_grid, find_offset, gate, peak_ratios, polish_base, symbol_bins,
};
use crate::lookup::{render, varidec};
use crate::{Mode, gray_decode};

/// fldigi 的回溯深度（`tracepair.trace`）。
const TRACEBACK: usize = 45;

/// 同一音调连续被判为最强多少次就当作窄带干扰（fldigi `CWI_MAXCOUNT`）。
const CWI_MAXCOUNT: i32 = 6;

/// 解码结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
  /// 正文（`\r` 换成换行，其余控制字符去掉；字节按 Latin-1 解读）。
  pub text: String,
  /// 使用的信号中心频率（Hz）：调用方没给就是自动检测的结果。
  pub center_hz: f64,
  /// 参与解码的符号数。
  pub symbols: usize,
  /// 译码用的交织深度（MFSK4 / 8 会在 5 与 10 之间择优，其余模式固定）。
  pub depth: usize,
}

/// 解码这个子模式用的符号窗口几何（MFSK 的音调间隔 = 波特率）。
fn grid_for(rate: u32, mode: &Mode) -> Option<Grid> {
  Grid::new(rate, mode.baud(), mode.baud(), mode.numtones())
}

/// 估计信号**最低音调**的频率（Hz）：见 [`crate::front::detect_base`]。
#[must_use]
pub fn detect_base(samples: &[f32], rate: u32, mode: &Mode) -> Option<f64> {
  detect_base_grid(samples, &grid_for(rate, mode)?)
}

/// 完整解码：音频 → 文本。
///
/// - `center`：信号中心频率（Hz）；`None` 时自动估计（[`detect_base`]）；
/// - `reverse`：边带反转（LSB 录音）。
///
/// 噪声与静音返回 `None`：滑动平均峰均比划不出信号区间、或区间的平均峰均比不够，就判无信号
/// （见 [`crate::front::gate`]）。
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
    Some(c) if c.is_finite() => (c - mode.bandwidth() / 2.0, false),
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

  let region: Vec<&[f64]> = bins.chunks_exact(nt).skip(first).take(span).collect();
  let mut best: Option<(i64, Decoded)> = None;
  for depth in depth_candidates(mode) {
    let mut rx = Receiver::new(mode, depth);
    let mut tmp = vec![0.0f64; nt];
    for chunk in &region {
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
    let text = render(&rx.chars);
    let score = readability(&text);
    // 严格大于：并列时留先试的（fldigi 的深度）。
    if best.as_ref().is_none_or(|(s, _)| score > *s) {
      best = Some((
        score,
        Decoded {
          text,
          center_hz: base + mode.bandwidth() / 2.0,
          symbols: span,
          depth,
        },
      ));
    }
  }
  best.map(|(_, d)| d)
}

fn decayavg(average: f64, input: f64, weight: i32) -> f64 {
  if weight <= 1 {
    return input;
  }
  (input - average) / f64::from(weight) + average
}

/// 判决 + 解交织 + 译码 + 切字符（fldigi `mfsk` 的接收链）。
struct Receiver {
  mode: Mode,
  inlv: Interleaver,
  inlv_depth: usize,
  dec1: Viterbi,
  dec2: Viterbi,
  /// 每个音调被连续判为最强的次数（窄带干扰检测）。
  cwi: Vec<i32>,
  /// 软符号的奇偶计数（`true` = 一对里的第一个）。
  first_of_pair: bool,
  pair: [u8; 2],
  met1: f64,
  met2: f64,
  datashreg: u32,
  chars: Vec<u16>,
}

impl Receiver {
  fn new(mode: &Mode, depth: usize) -> Self {
    Self {
      mode: *mode,
      inlv: Interleaver::new(mode.symbits, depth, false),
      inlv_depth: depth,
      dec1: Viterbi::new(Code::NASA_K7, TRACEBACK),
      dec2: Viterbi::new(Code::NASA_K7, TRACEBACK),
      cwi: vec![0; mode.numtones()],
      first_of_pair: false,
      pair: [0; 2],
      met1: 0.0,
      met2: 0.0,
      datashreg: 1,
      chars: Vec::new(),
    }
  }

  /// fldigi `harddecode`：最强音调，以及「所有音调都过强」的宽带脉冲（staticburst）。
  fn hard_decode(bins: &[f64]) -> (usize, bool) {
    let n = bins.len() as f64;
    let avg = (bins.iter().sum::<f64>() / n).max(1e-20);
    let mut best = (0.0f64, 0usize);
    let mut burst = 0usize;
    for (i, &x) in bins.iter().enumerate() {
      if x > best.0 {
        best = (x, i);
      }
      if x > 2.0 * avg {
        burst += 1;
      }
    }
    (best.1, burst == bins.len())
  }

  /// fldigi `softdecode`：逐音调幅度按格雷码折成每个码位的软值（0..=255，128 = 无把握）。
  fn soft_decode(&mut self, bins: &[f64], current: usize, static_burst: bool) -> Vec<u8> {
    let (n, bits) = (bins.len(), self.mode.symbits);
    let mut sum = 0.0;
    for (i, &x) in bins.iter().enumerate() {
      if self.cwi[i] < CWI_MAXCOUNT {
        sum += x;
      }
    }
    let avg = sum / n as f64;
    // 避免后面除零。
    let sum = sum.max(1e-10);
    // 动态窄带干扰：用硬判决的结果给每个音调的「连续最强」计数加减，从 1 起（0 号音调不参与）。
    for k in 1..n {
      if k == current {
        self.cwi[k] += 1;
      } else {
        self.cwi[k] -= 1;
      }
      self.cwi[k] = self.cwi[k].clamp(0, CWI_MAXCOUNT + 1);
    }
    let mut b = vec![0.0f64; bits];
    for (i, &mag) in bins.iter().enumerate() {
      let j = gray_decode(i as u32) as usize;
      let binmag = if self.cwi[i] > CWI_MAXCOUNT {
        avg // 窄带干扰：软删除成平均电平
      } else if i == current {
        2.0 * mag // 硬判决也投一票
      } else {
        mag
      };
      for (k, bk) in b.iter_mut().enumerate() {
        *bk += if j & (1 << (bits - k - 1)) != 0 {
          binmag
        } else {
          -binmag
        };
      }
    }
    b.iter()
      .map(|&bk| {
        if static_burst {
          PUNCTURE
        } else {
          (128.0 + bk / sum * 256.0).clamp(0.0, 255.0) as u8
        }
      })
      .collect()
  }

  /// 一个符号的各音调幅度 → 若干字符（写进 `self.chars`）。
  fn feed(&mut self, bins: &[f64]) {
    let (current, burst) = Self::hard_decode(bins);
    let mut syms = self.soft_decode(bins, current, burst);
    self.inlv.symbols(&mut syms);
    for s in syms {
      self.decode_symbol(s);
    }
  }

  /// 信号结束后把管线里压着的字符推出来：喂若干个「全是删除位」的符号。
  ///
  /// 反向交织器与 Viterbi 回溯合起来有一段固定延迟（`(符号位数 − 1) × 深度` 个符号加 `回溯 / 每符号的
  /// 译码位数` 个符号），信号最后那几个字符要等这么多个符号才流得出来。fldigi 的接收机收完信号后
  /// 还会一直跑（噪声会把它们推出来），离线文件没有「后面」，所以在这里补。删除位的度量增量为 0，
  /// 不会改变已有路径的取舍，只是让回溯窗口往前走。
  fn flush(&mut self) {
    let bits = self.mode.symbits;
    // 每个符号含 `bits / 2` 对码位 = 这么多个译码位；向上取整。
    let tail = (TRACEBACK * 2).div_ceil(bits);
    for _ in 0..(bits - 1) * self.inlv_depth + tail {
      let mut syms = vec![PUNCTURE; bits];
      self.inlv.symbols(&mut syms);
      for s in syms {
        self.decode_symbol(s);
      }
    }
  }

  /// fldigi `decodesymbol`：软符号两两配对进 Viterbi。
  ///
  /// 奇数位的模式（5 位 / 3 位）码位对与符号边界错开，两个译码器交替喂入，
  /// 取路径度量滑动平均更高的那个的输出；偶数位只用一个译码器。
  fn decode_symbol(&mut self, symbol: u8) {
    self.pair = [self.pair[1], symbol];
    self.first_of_pair = !self.first_of_pair;
    let bit = if self.mode.symbits % 2 == 1 {
      if self.first_of_pair {
        let (c, met) = self.dec1.decode(self.pair);
        self.met1 = decayavg(self.met1, f64::from(met), 50);
        if self.met1 < self.met2 {
          return;
        }
        c
      } else {
        let (c, met) = self.dec2.decode(self.pair);
        self.met2 = decayavg(self.met2, f64::from(met), 50);
        if self.met2 < self.met1 {
          return;
        }
        c
      }
    } else {
      if self.first_of_pair {
        return;
      }
      let (c, met) = self.dec2.decode(self.pair);
      self.met2 = decayavg(self.met2, f64::from(met), 50);
      c
    };
    self.recv_bit(bit);
  }

  /// fldigi `recvbit`：Varicode 码字以 `001` 结尾切分。
  fn recv_bit(&mut self, bit: bool) {
    self.datashreg = (self.datashreg << 1) | u32::from(bit);
    if self.datashreg & 7 == 1 {
      if let Some(c) = varidec(self.datashreg >> 1) {
        self.chars.push(c);
      }
      self.datashreg = 1;
    }
  }
}

/// 译码时要试的交织深度。
///
/// MFSK4 / 8 在 fldigi 里是 5，但 gMFSK（这个模式的原作者实现）发出的 MFSK8 录音实测用 10 才能译出
/// 原文（用 gmfsk.connect.fi 的官方样本验证过），所以这两个模式两种都试、按可读性择优。
/// 其余模式两家一致，只有一个深度。
fn depth_candidates(mode: &Mode) -> Vec<usize> {
  let mut v = vec![mode.depth];
  if matches!(mode.name, "4" | "8") {
    v.push(10);
  }
  v
}

/// 文本可读性：可打印 ASCII（含空格与换行）加分，其余扣双倍。用来在多个候选译码之间选一个。
fn readability(text: &str) -> i64 {
  text
    .chars()
    .map(|c| {
      if c.is_ascii_graphic() || c == ' ' || c == '\n' {
        1
      } else {
        -2
      }
    })
    .sum()
}
