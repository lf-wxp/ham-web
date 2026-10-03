//! SSTV（Slow-Scan Television）慢扫描电视图像解码 DSP 引擎。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! 处理链路：
//! 1. WAV 解析（复用 [`ham_web_apt::parse_wav`]）；
//! 2. 重采样到工作采样率 [`WORK_RATE`]；
//! 3. [`fm`] —— Hilbert 变换 + 相位差分做 FM 解调，得到瞬时频率；
//! 4. [`vis`] —— 检测 VIS 头，识别 Martin / Scottie / Robot 模式；
//! 5. [`decode`] —— 逐行锁定同步脉冲、采样并重建 RGB 图像。

mod decode;
mod fm;
mod modes;
mod resample;
mod vis;

use std::fmt;

pub use decode::SstvImage;
pub use modes::{ColorKind, MODES, Mode};

/// 工作采样率（Hz）。
pub const WORK_RATE: u32 = 8000;

/// 频率基准（Hz）。
pub const SYNC_HZ: f32 = 1200.0;
pub const BLACK_HZ: f32 = 1500.0;
pub const WHITE_HZ: f32 = 2300.0;

/// 解码错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
  /// 不是有效的 WAV 文件。
  NotWav,
  /// 音频格式不受支持。
  UnsupportedFormat,
  /// 音频过短。
  TooShort,
  /// 未检测到 VIS 头（不是 SSTV 信号）。
  NoVis,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let msg = match self {
      Self::NotWav => "不是有效的 WAV 文件",
      Self::UnsupportedFormat => "不支持的音频格式（仅支持 PCM / IEEE float WAV）",
      Self::TooShort => "音频过短，无法解码出图像",
      Self::NoVis => "未检测到 VIS 头，请确认是 SSTV 音频（WAV）",
    };
    f.write_str(msg)
  }
}

impl std::error::Error for Error {}

/// 解码一段 WAV 音频字节，返回重建的 RGB 图像。
pub fn decode(bytes: &[u8]) -> Result<SstvImage, Error> {
  let (samples, rate) = ham_web_apt::parse_wav(bytes).map_err(|_| Error::NotWav)?;
  decode_samples(&samples, rate)
}

/// 从已归一化的单声道样本（[-1, 1]）与采样率解码。
pub fn decode_samples(samples: &[f32], rate: u32) -> Result<SstvImage, Error> {
  if samples.is_empty() || rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  let working = resample::to_work_rate(samples, rate);
  if working.is_empty() {
    return Err(Error::TooShort);
  }
  let freq = fm::fm_demodulate(&working, WORK_RATE);
  let (code, vis_end) = vis::detect(&freq).ok_or(Error::NoVis)?;
  let mode = modes::by_vis_code(code).ok_or(Error::NoVis)?;
  decode::assemble(&freq, mode, vis_end, rate)
}
