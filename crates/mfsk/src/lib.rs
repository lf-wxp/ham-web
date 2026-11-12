//! MFSK 数字模式编解码（MFSK4 / 8 / 11 / 16 / 22 / 31 / 32 / 64 / 128）。
//!
//! 算法转写自 **fldigi**（GPL-3.0）`src/mfsk/mfsk.cxx`（W1HKJ，改编自 gMFSK，
//! © 2001–2003 Tomi Manninen OH2BNS）；MFSK 的设计与 Varicode © IZ8BLY。
//!
//! 结构（以 MFSK16 为例）：
//! - 16 个音调、音调间隔 = 波特率 = 15.625 Hz（`8000 / 512`）；一个符号 = 4 个码位；
//! - 发送：Varicode → 码率 1/2 卷积码（K = 7）→ `size = 符号位数` 的卷积交织器 → 格雷码选音调；
//! - 接收：每符号对每个音调做 Goertzel（窗口恰为一个符号，音调正交）→ 逐音调幅度按格雷码
//!   折成软位 → 反向交织 → Viterbi → 按 `001` 切字符；
//! - 5 位 / 3 位符号的模式（MFSK4 / 8 / 31）码位对与符号边界错开，fldigi 用两个 Viterbi
//!   交替喂入、按路径度量投票，这里同样。
//!
//! 与 fldigi 的差异（接收端只保留「对离线文件有用」的部分）：
//! - 符号同步：fldigi 用滑动 FFT 逐样本跟踪；这里在整段录音上搜一次对齐偏移，不跟踪时钟漂移；
//! - 没有 AFC（载频偏差由整段录音上的一次中心频率搜索代替）、没有图片模式；
//! - 解调用 Goertzel 而不是滑动 FFT（幅度的比例不变，判决只看比值）。

use ham_web_dsp::tone_power;

pub mod fec;
pub mod front;
pub mod lookup;
pub mod varicode;

mod rx;
mod tx;

pub use rx::{Decoded, decode_audio, detect_base};
pub use tx::{encode_audio, encode_symbols};

/// 一个 MFSK 子模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
  pub name: &'static str,
  /// fldigi 里该模式的标称采样率（只用来算波特率；实际音频可以是任何采样率）。
  pub rate: u32,
  /// 标称采样率下一个符号的样本数。
  pub symlen: usize,
  /// 每符号码位数（音调数 = `2^symbits`）。
  pub symbits: usize,
  /// 交织深度。
  pub depth: usize,
  /// 发送前导 / 收尾冲刷长度（码位）。
  pub preamble: usize,
}

impl Mode {
  /// 音调数。
  #[must_use]
  pub fn numtones(&self) -> usize {
    1 << self.symbits
  }

  /// 波特率 = 音调间隔（Hz）。
  #[must_use]
  pub fn baud(&self) -> f64 {
    f64::from(self.rate) / self.symlen as f64
  }

  /// 信号带宽（最低音调到最高音调，Hz）。
  #[must_use]
  pub fn bandwidth(&self) -> f64 {
    (self.numtones() - 1) as f64 * self.baud()
  }
}

/// 支持的子模式（fldigi `mfsk.cxx` 构造函数里的表；不含 64L / 128L 两个长交织变体）。
pub const MODES: &[Mode] = &[
  Mode {
    name: "4",
    rate: 8000,
    symlen: 2048,
    symbits: 5,
    depth: 5,
    preamble: 107,
  },
  Mode {
    name: "8",
    rate: 8000,
    symlen: 1024,
    symbits: 5,
    depth: 5,
    preamble: 107,
  },
  Mode {
    name: "11",
    rate: 11025,
    symlen: 1024,
    symbits: 4,
    depth: 10,
    preamble: 107,
  },
  Mode {
    name: "16",
    rate: 8000,
    symlen: 512,
    symbits: 4,
    depth: 10,
    preamble: 107,
  },
  Mode {
    name: "22",
    rate: 11025,
    symlen: 512,
    symbits: 4,
    depth: 10,
    preamble: 107,
  },
  Mode {
    name: "31",
    rate: 8000,
    symlen: 256,
    symbits: 3,
    depth: 10,
    preamble: 107,
  },
  Mode {
    name: "32",
    rate: 8000,
    symlen: 256,
    symbits: 4,
    depth: 10,
    preamble: 107,
  },
  Mode {
    name: "64",
    rate: 8000,
    symlen: 128,
    symbits: 4,
    depth: 10,
    preamble: 180,
  },
  Mode {
    name: "128",
    rate: 8000,
    symlen: 64,
    symbits: 4,
    depth: 20,
    preamble: 214,
  },
];

/// 解析 `"16"` 形式的子模式名。
#[must_use]
pub fn parse_mode(name: &str) -> Option<&'static Mode> {
  let name = name.trim();
  MODES.iter().find(|m| m.name == name)
}

/// 解码这个子模式所需的最低采样率（Hz）：信号带加上两侧各一个音调间隔的邻位，必须放得进 Nyquist
/// （自动检测中心频率时还要多留两个音调的余量，所以按 `带宽 + 4 个间隔` 算）。
#[must_use]
pub fn min_sample_rate(mode: &Mode) -> u32 {
  (2.0 * (mode.bandwidth() + 4.0 * mode.baud())).ceil() as u32 + 1
}

/// 中心频率能否放下整个信号带（不越过 0 与 Nyquist，含两侧各一个音调间隔的余量）。
#[must_use]
pub fn center_in_range(mode: &Mode, rate: u32, center_hz: f64) -> bool {
  let low = center_hz - mode.bandwidth() / 2.0;
  center_hz.is_finite()
    && low >= 0.0
    && low + (mode.numtones() as f64 + 1.0) * mode.baud() < f64::from(rate) / 2.0
}

/// 符号值 → 音调号（fldigi `grayencode`）。
///
/// 名字沿用 fldigi，但**实现是前缀异或**（传统叫法里的「格雷码 → 二进制」），与传统的
/// `x ^ (x >> 1)` 方向相反；接收端的 [`gray_decode`] 才是 `x ^ (x >> 1)`。两者互为逆运算，
/// 相邻音调对应的符号值只差一位 —— 这正是用格雷码的目的。
#[must_use]
pub fn gray_encode(data: u32) -> u32 {
  let mut bits = data;
  let mut d = data >> 1;
  while d != 0 {
    bits ^= d;
    d >>= 1;
  }
  bits
}

/// 音调号 → 符号值（fldigi `graydecode`，`x ^ (x >> 1)`）。
#[must_use]
pub fn gray_decode(data: u32) -> u32 {
  data ^ (data >> 1)
}

/// 窗口 `win` 个样本、起点 `start`（样本位置，四舍五入）处各音调的幅度。
///
/// 窗口越过末尾返回 `false`。音调 `k` 的频率 = `base + k × spacing`。
pub(crate) fn tone_magnitudes(
  samples: &[f32],
  rate: u32,
  start: f64,
  win: usize,
  base: f64,
  spacing: f64,
  out: &mut [f64],
) -> bool {
  if !start.is_finite() || start < 0.0 {
    return false;
  }
  let s = start.round() as usize;
  let Some(window) = samples.get(s..s.saturating_add(win)) else {
    return false;
  };
  for (k, o) in out.iter_mut().enumerate() {
    // Goertzel 的功率在近零时可能被舍入成极小的负数，开方前钳到 0，否则会冒出 NaN。
    *o = tone_power(window, rate, base + k as f64 * spacing)
      .max(0.0)
      .sqrt();
  }
  true
}
