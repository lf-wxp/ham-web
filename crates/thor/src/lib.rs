//! THOR 数字模式编解码（THOR micro / 4 / 5 / 8 / 11 / 16 / 22 / 25x4 / 50x1 / 50x2 / 100）。
//!
//! 算法转写自 **fldigi**（GPL-3.0）`src/thor/thor.cxx`（W1HKJ、VK2ETA、KL4YFD；
//! 模式设计 © Nino Porcino IZ8BLY / Murray Greenman ZL1BPU）。
//!
//! 结构：
//! - 18 个音调，下一个音调 = `(当前音调 + 2 + nibble) mod 18` —— 与 DominoEX 同一套 **IFK+**
//!   增量频移键控；区别在 THOR **全程带前向纠错**；
//! - 发送：THOR Varicode（主字符集 = IZ8BLY MFSK Varicode，二次字符集用 MFSK 码表里没分配的 12 位码）
//!   → 码率 1/2 卷积码（慢速模式 K = 7，高速模式 K = 15）→ `size = 4` 的卷积交织器 → 4 个码位一个
//!   nibble → IFK+ 选音调；
//! - 接收：每符号对 18 个音调做 Goertzel → 取最强音调 → 相邻符号的音调差还原 nibble →
//!   拆成 4 个码位软值 → 反向交织 → Viterbi → 按 `001` 切字符。
//!
//! 与 fldigi 的差异（接收端只保留「对离线文件有用」的部分）：
//! - 判决用 fldigi 的**硬位路径**（`decodesymbol`：音调差落在合法范围就给 255 / 0，越界或宽带脉冲给
//!   128 删除位）；fldigi 默认的软位路径（`softdecodesymbol`）按载频偏差给软值，对干净信号与硬位路径
//!   的输出相同，弱信号下略胜；
//! - 前端同 MFSK：整段录音搜一次符号对齐与中心频率（[`ham_web_mfsk::front`]），不跟踪漂移，没有 AFC，
//!   没有 CWI 抑制、前导检测与图片模式。

pub mod varicode;

mod rx;
mod tx;

pub use rx::{Decoded, decode_audio, detect_base};
pub use tx::{encode_audio, encode_symbols};

use ham_web_mfsk::fec::Code;

/// 音调数。
pub const NUMTONES: usize = 18;

/// 一个 THOR 子模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
  pub name: &'static str,
  /// fldigi 里该模式的标称采样率（只用来算波特率；实际音频可以是任何采样率）。
  pub rate: u32,
  /// 标称采样率下一个符号的样本数。
  pub symlen: usize,
  /// 音调间隔是波特率的几倍（fldigi `doublespaced`）。
  pub spacing_factor: usize,
  /// 交织深度。
  pub depth: usize,
  /// 收尾冲刷的空闲符个数。
  pub flush: usize,
  /// 高速模式用 K = 15 的长约束卷积码。
  pub k15: bool,
  /// THOR micro 不发 STX / EOT 帧头帧尾。
  pub micro: bool,
}

impl Mode {
  /// 波特率（Hz）。
  #[must_use]
  pub fn baud(&self) -> f64 {
    f64::from(self.rate) / self.symlen as f64
  }

  /// 音调间隔（Hz）。
  #[must_use]
  pub fn spacing(&self) -> f64 {
    self.baud() * self.spacing_factor as f64
  }

  /// 信号带宽（fldigi 的 `bandwidth = 18 × 音调间隔`）。
  #[must_use]
  pub fn bandwidth(&self) -> f64 {
    NUMTONES as f64 * self.spacing()
  }

  /// 音调 `k` 相对信号中心的频偏（Hz）：`(k − 8.5) × 间隔`，即 18 个音调关于中心对称。
  #[must_use]
  pub fn tone_offset(&self, k: usize) -> f64 {
    (k as f64 - (NUMTONES as f64 - 1.0) / 2.0) * self.spacing()
  }

  /// 卷积码参数。
  #[must_use]
  pub fn code(&self) -> Code {
    if self.k15 {
      Code::IEEE_K15
    } else {
      Code::NASA_K7
    }
  }

  /// Viterbi 回溯深度（fldigi：K = 7 取 45，K = 15 取 `15 × 12`）。
  #[must_use]
  pub fn traceback(&self) -> usize {
    if self.k15 { 15 * 12 } else { 45 }
  }
}

/// 支持的子模式（fldigi `thor.cxx` 构造函数里的表）。
pub const MODES: &[Mode] = &[
  Mode {
    name: "micro",
    rate: 8000,
    symlen: 4000,
    spacing_factor: 1,
    depth: 4,
    flush: 4,
    k15: false,
    micro: true,
  },
  Mode {
    name: "4",
    rate: 8000,
    symlen: 2048,
    spacing_factor: 2,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "5",
    rate: 11025,
    symlen: 2048,
    spacing_factor: 2,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "8",
    rate: 8000,
    symlen: 1024,
    spacing_factor: 2,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "11",
    rate: 11025,
    symlen: 1024,
    spacing_factor: 1,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "16",
    rate: 8000,
    symlen: 512,
    spacing_factor: 1,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "22",
    rate: 11025,
    symlen: 512,
    spacing_factor: 1,
    depth: 10,
    flush: 4,
    k15: false,
    micro: false,
  },
  Mode {
    name: "25x4",
    rate: 8000,
    symlen: 320,
    spacing_factor: 4,
    depth: 50,
    flush: 40,
    k15: true,
    micro: false,
  },
  Mode {
    name: "50x1",
    rate: 8000,
    symlen: 160,
    spacing_factor: 1,
    depth: 50,
    flush: 40,
    k15: true,
    micro: false,
  },
  Mode {
    name: "50x2",
    rate: 8000,
    symlen: 160,
    spacing_factor: 2,
    depth: 50,
    flush: 40,
    k15: true,
    micro: false,
  },
  Mode {
    name: "100",
    rate: 8000,
    symlen: 80,
    spacing_factor: 1,
    depth: 50,
    flush: 40,
    k15: true,
    micro: false,
  },
];

/// 解析 `"16"` / `"50x2"` / `"micro"` 形式的子模式名。
#[must_use]
pub fn parse_mode(name: &str) -> Option<&'static Mode> {
  let name = name.trim();
  MODES.iter().find(|m| m.name == name)
}

/// 解码这个子模式所需的最低采样率（Hz）：信号带加上两侧各一个音调间隔的邻位，必须放得进 Nyquist
/// （自动检测中心频率时还要多留两个音调的余量，所以按 `带宽 + 4 个间隔` 算）。
#[must_use]
pub fn min_sample_rate(mode: &Mode) -> u32 {
  (2.0 * (mode.bandwidth() + 4.0 * mode.spacing())).ceil() as u32 + 1
}

/// 中心频率能否放下整个信号带（不越过 0 与 Nyquist，含两侧各一个音调间隔的余量）。
#[must_use]
pub fn center_in_range(mode: &Mode, rate: u32, center_hz: f64) -> bool {
  let low = center_hz + mode.tone_offset(0);
  center_hz.is_finite()
    && low >= 0.0
    && low + (NUMTONES as f64 + 1.0) * mode.spacing() < f64::from(rate) / 2.0
}
