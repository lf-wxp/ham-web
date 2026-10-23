//! SSTV（Slow-Scan Television）慢扫描电视图像解码 DSP 引擎。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! 处理链路：
//! 1. WAV 解析（复用 [`ham_web_wav::parse`]）；
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
pub use modes::{ColorKind, Comp, EdgeEvidence, MODES, Mode, ROBOT_36_SEPARATOR, Scan};

/// 工作采样率（Hz）。
pub const WORK_RATE: u32 = 8000;

/// 频率基准（Hz）。
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
  /// 音频文件数据不完整。
  Truncated,
  /// 文件中没有可解码的音频数据。
  NoAudio,
}

impl From<ham_web_wav::Error> for Error {
  fn from(e: ham_web_wav::Error) -> Self {
    match e {
      ham_web_wav::Error::NotWav => Self::NotWav,
      ham_web_wav::Error::UnsupportedFormat => Self::UnsupportedFormat,
      ham_web_wav::Error::Truncated => Self::Truncated,
      ham_web_wav::Error::NoAudio => Self::NoAudio,
    }
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let msg = match self {
      Self::NotWav => "不是有效的 WAV 文件",
      Self::UnsupportedFormat => "不支持的音频格式（仅支持 PCM / IEEE float WAV）",
      Self::TooShort => "音频过短，无法解码出图像",
      Self::NoVis => "未检测到 VIS 头，请确认是 SSTV 音频（WAV）",
      Self::Truncated => "音频文件数据不完整",
      Self::NoAudio => "文件中没有可解码的音频数据",
    };
    f.write_str(msg)
  }
}

impl Error {
  /// 稳定的机器可读错误码（同 `ham_web_wav::Error::code`）。
  #[must_use]
  pub fn code(&self) -> &'static str {
    match self {
      Self::NotWav => "not-wav",
      Self::UnsupportedFormat => "unsupported-format",
      Self::TooShort => "too-short",
      Self::NoVis => "no-vis",
      Self::Truncated => "truncated",
      Self::NoAudio => "no-audio",
    }
  }
}

impl std::error::Error for Error {}

/// 解码一段 WAV 音频字节，返回重建的 RGB 图像。
pub fn decode(bytes: &[u8]) -> Result<SstvImage, Error> {
  decode_with_progress(bytes, &mut |_| {})
}

/// 与 [`decode`] 相同，但按行回报进度（0..1）。
///
/// **当前没有调用方（预留）**：Worker 同步执行期间，投递给主线程的 `postMessage` 到不了
/// 页面（2026-10-23 实测：release / debug 两版 wasm 上都是 0 条，结果信封却正常到达），
/// 所以界面侧只用前端自算的「已用时」。保留它是因为逐行回调本身是对的、有真实录音单测守着；
/// 等解码改成「分段往返」
/// （worker 每解一块就返回、页面再发下一块）时，这里就是现成的分块边界。
/// 回调只在逐行扫描阶段触发；FM 解调与重采样是一次性数组运算，没有可切分的边界。
pub fn decode_with_progress(
  bytes: &[u8],
  on_progress: &mut dyn FnMut(f32),
) -> Result<SstvImage, Error> {
  let (samples, rate) = ham_web_wav::parse(bytes)?;
  decode_samples_with_progress(&samples, rate, on_progress)
}

/// 从已归一化的单声道样本（[-1, 1]）与采样率解码。
pub fn decode_samples(samples: &[f32], rate: u32) -> Result<SstvImage, Error> {
  decode_samples_with_progress(samples, rate, &mut |_| {})
}

/// 与 [`decode_samples`] 相同，但按行回报进度。
pub fn decode_samples_with_progress(
  samples: &[f32],
  rate: u32,
  on_progress: &mut dyn FnMut(f32),
) -> Result<SstvImage, Error> {
  if samples.is_empty() || rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  let working = resample::to_work_rate(samples, rate)?;
  if working.is_empty() {
    return Err(Error::TooShort);
  }
  let freq = fm::fm_demodulate(&working, WORK_RATE);
  let vis = vis::detect(&freq).ok_or(Error::NoVis)?;
  let mode = modes::by_vis_code(vis.code).ok_or(Error::NoVis)?;
  decode::assemble_with_progress(&freq, mode, vis.end, rate, on_progress)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 逐行进度必须是单调的，并且走到 1.0 —— 界面的进度条依赖这两个性质。
  ///
  /// 用仓里的真实录音（Martin M1，115.8 秒）而不是合成信号：行同步、行数、提前 break
  /// 这些边界只有真实样本才会走到。
  #[test]
  fn progress_is_monotonic_and_reaches_the_end() {
    let bytes = include_bytes!("../samples/martin_m1_sunset.wav");
    let mut seen: Vec<f32> = Vec::new();
    let img = decode_with_progress(bytes, &mut |p| seen.push(p)).expect("真实录音应能解码");
    assert!(seen.len() > 100, "回调次数太少：{}", seen.len());
    assert!(
      seen.windows(2).all(|w| w[0] <= w[1] + f32::EPSILON),
      "进度不是单调递增"
    );
    let last = *seen.last().expect("至少回调一次");
    assert!((last - 1.0).abs() < 0.02, "最后一条进度是 {last}");
    assert_eq!(img.height, 256);
  }
}
