//! NOAA APT（Automatic Picture Transmission，自动图像传输）气象卫星图像解码 DSP 引擎。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! 完整处理链路：
//! 1. [`wav`] —— 解析 WAV（PCM / IEEE float，单/双声道，8/16/24/32 位）为归一化单声道样本；
//! 2. [`resample`] —— 线性重采样到工作采样率 [`WORK_RATE`]；
//! 3. [`demod`] —— 对 2400 Hz 副载波做正交 AM 解调，取出视频包络；
//! 4. [`sync`] —— 在字速率包络上检测 sync A（1040 Hz），定位每行起点；
//! 5. [`image`] —— 按行提取 909×2 像素并借助遥测楔形校准，重建双通道灰度图。
//!
//! # APT 格式速览
//!
//! - 副载波 2400 Hz，AM 调制（视频倒置：幅度越大越黑）；
//! - 行速率 2 行/秒，每行 2080 字（4160 字/秒）；
//! - 每行两个通道（A/B），各 1040 字：sync 39 + space 47 + 图像 909 + 遥测 45。

mod demod;
mod filter;
mod image;
mod resample;
mod sync;
mod wav;

use std::fmt;

pub use image::AptImage;

/// 副载波频率（Hz）。
pub const SUBCARRIER_HZ: f32 = 2400.0;
/// 字速率（字/秒），即视频采样率。
pub const WORD_RATE: u32 = 4160;
/// 每行字数（0.5 秒）。
pub const LINE_WORDS: usize = 2080;
/// 每通道字数。
pub const CHANNEL_WORDS: usize = 1040;
/// 同步字数。
pub const SYNC_WORDS: usize = 39;
/// 间隔（space）字数。
pub const SPACE_WORDS: usize = 47;
/// 每通道图像像素数。
pub const IMAGE_WORDS: usize = 909;
/// 每通道遥测楔形字数。
pub const TELEMETRY_WORDS: usize = 45;
/// sync A 频率（Hz）= 字速率 / 4。
pub const SYNC_A_HZ: f32 = 1040.0;
/// sync B 频率（Hz）= 字速率 / 5。
pub const SYNC_B_HZ: f32 = 832.0;
/// 解调工作采样率（5 × 字速率，兼顾同步分辨率与运算量）。
pub const WORK_RATE: u32 = 20_800;

/// 解码错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
  /// 不是有效的 WAV 文件。
  NotWav,
  /// 音频格式不受支持（仅支持 PCM / IEEE float）。
  UnsupportedFormat,
  /// 音频数据不完整。
  Truncated,
  /// 文件中没有可解码的音频数据。
  NoAudio,
  /// 音频过短，无法解码出图像。
  TooShort,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let msg = match self {
      Self::NotWav => "不是有效的 WAV 文件",
      Self::UnsupportedFormat => "不支持的音频格式（仅支持 PCM / IEEE float WAV）",
      Self::Truncated => "音频文件数据不完整",
      Self::NoAudio => "文件中没有可解码的音频数据",
      Self::TooShort => "音频过短，无法解码出图像",
    };
    f.write_str(msg)
  }
}

impl std::error::Error for Error {}

/// 解码一段 WAV 音频字节，返回重建的双通道灰度图像。
///
/// 输入可为任意采样率、单/双声道的 PCM 或 IEEE float WAV；输出为 909 像素宽的
/// 通道 A / 通道 B 灰度图（高度等于检测到的行数）。
pub fn decode(bytes: &[u8]) -> Result<AptImage, Error> {
  let (samples, rate) = wav::parse(bytes)?;
  decode_samples(&samples, rate)
}

/// 从已归一化的单声道样本（[-1, 1]）与采样率解码。
pub fn decode_samples(samples: &[f32], sample_rate: u32) -> Result<AptImage, Error> {
  if sample_rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  // 至少约 1 秒（2 行）才可能解码出图像。
  if (samples.len() as f64 / sample_rate as f64) < 1.0 {
    return Err(Error::TooShort);
  }
  let working = resample::to_rate(samples, sample_rate, WORK_RATE)?;
  let envelope = demod::am_demodulate(&working, WORK_RATE, SUBCARRIER_HZ);
  let words = demod::to_words(&envelope);
  let lines = sync::find_lines(&words);
  image::assemble(&words, &lines, sample_rate)
}

// ── 分阶段 API ────────────────────────────────────────────────────────────
// 供需要让出主线程 / 展示进度的调用方逐步执行，等价于 [`decode`] 的拆分版本。

/// 解析 WAV 为归一化单声道样本与采样率。
pub fn parse_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), Error> {
  wav::parse(bytes)
}

/// 重采样到工作采样率 [`WORK_RATE`]。
pub fn resample(samples: &[f32], source_rate: u32) -> Result<Vec<f32>, Error> {
  resample::to_rate(samples, source_rate, WORK_RATE)
}

/// 对工作采样率的样本做 AM 解调，返回视频包络（采样率仍为 [`WORK_RATE`]）。
pub fn demodulate(samples: &[f32]) -> Vec<f32> {
  demod::am_demodulate(samples, WORK_RATE, SUBCARRIER_HZ)
}

/// 将包络降采样到字速率。
pub fn to_words(envelope: &[f32]) -> Vec<f32> {
  demod::to_words(envelope)
}

/// 在字速率包络上检测每行起点。
pub fn find_lines(words: &[f32]) -> Vec<usize> {
  sync::find_lines(words)
}

/// 由字速率包络与行起点组装双通道灰度图。
pub fn assemble(words: &[f32], lines: &[usize], source_rate: u32) -> Result<AptImage, Error> {
  image::assemble(words, lines, source_rate)
}
