//! WAV（RIFF）解析：支持 PCM 与 IEEE float，单/双声道，8/16/24/32 位。
//!
//! 输出归一化到 [-1, 1] 的单声道样本（多声道取平均）与原始采样率。

use std::fmt;

/// WAV 解析错误。
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
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::NotWav => "不是有效的 WAV 文件",
      Self::UnsupportedFormat => "不支持的音频格式（仅支持 PCM / IEEE float WAV）",
      Self::Truncated => "音频文件数据不完整",
      Self::NoAudio => "文件中没有可解码的音频数据",
    })
  }
}

impl Error {
  /// 稳定的机器可读错误码：Worker 回信用它而不是 `Display` 的中文文案，
  /// 页面按码取各语言的词条（见 `ham-web-app` 的 `decode_worker::localize_error`）。
  #[must_use]
  pub fn code(&self) -> &'static str {
    match self {
      Self::NotWav => "not-wav",
      Self::UnsupportedFormat => "unsupported-format",
      Self::Truncated => "truncated",
      Self::NoAudio => "no-audio",
    }
  }
}

impl std::error::Error for Error {}

/// 解析 WAV 字节，返回 `(归一化单声道样本, 采样率)`。
pub fn parse(bytes: &[u8]) -> Result<(Vec<f32>, u32), Error> {
  let (fmt, data) = scan_chunks(bytes)?;
  let fmt = fmt.ok_or(Error::NotWav)?;
  let data = data.ok_or(Error::NoAudio)?;
  if fmt.len() < 16 {
    return Err(Error::UnsupportedFormat);
  }

  let format = u16::from_le_bytes([fmt[0], fmt[1]]);
  // `channels = 0` 的畸形头按单声道解析而不是拒绝：这样的文件在真实录音机里
  // 偶有出现（写头不完整），宽容处理能多解一批。
  let channels = u16::from_le_bytes([fmt[2], fmt[3]]).max(1) as usize;
  let rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
  let bits = u16::from_le_bytes([fmt[14], fmt[15]]);

  if rate == 0 {
    return Err(Error::UnsupportedFormat);
  }

  let samples = match format {
    // 显式校验位深，避免非标准位深（如 20 位）被静默误解析。
    1 if matches!(bits, 8 | 16 | 24 | 32) => decode_pcm(data, bits, channels),
    3 if matches!(bits, 32 | 64) => decode_float(data, bits, channels),
    _ => return Err(Error::UnsupportedFormat),
  };

  if samples.is_empty() {
    return Err(Error::NoAudio);
  }
  Ok((samples, rate))
}

/// RIFF 块扫描结果：`(fmt 块, data 块)`，两者都可能缺失。
type Chunks<'a> = (Option<&'a [u8]>, Option<&'a [u8]>);

/// 扫描 RIFF 块，返回 `(fmt 块, data 块)`。
///
/// 块长度直接取自文件头，而 wasm32 上 `usize` 是 32 位：`body + size` 会回绕，让
/// 「是否超出文件」的比较失效，紧接着的切片就会 panic —— 畸形文件能把解码 Worker
/// 打成 trap，用户拿到的是英文 trap 文本而不是 `truncated`。所以这里用
/// `checked_add`，data 块按实际可读范围夹取（部分文件在 data 块后有补齐或截断）。
///
/// RIFF 头里的总长字段（bytes 4..8）**不参与校验**：很多工具写的总长与实际不一致，
/// 逐块扫描反而对它们更宽容 —— 有意不校验，不是遗漏。
fn scan_chunks(bytes: &[u8]) -> Result<Chunks<'_>, Error> {
  if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    return Err(Error::NotWav);
  }
  let mut fmt: Option<&[u8]> = None;
  let mut data: Option<&[u8]> = None;
  let mut pos = 12usize;
  while let Some(header) = bytes.get(pos..pos + 8) {
    let id = &header[0..4];
    let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let body = pos + 8;
    match body.checked_add(size).filter(|&end| end <= bytes.len()) {
      Some(end) => {
        match id {
          b"fmt " => fmt = Some(&bytes[body..end]),
          b"data" => {
            data = Some(&bytes[body..end]);
            break;
          }
          _ => {}
        }
        // 奇数大小的块会补齐 1 字节对齐。
        pos = end + (size & 1);
      }
      None => {
        if id == b"data" {
          data = Some(&bytes[body..]);
          break;
        }
        return Err(Error::Truncated);
      }
    }
  }
  Ok((fmt, data))
}

/// 保留声道交错（不混音）的解析 —— 多声道信号（如 LRPT 的立体声 I/Q）
/// 需要原始交错样本而不是平均成单声道。
pub fn parse_interleaved(bytes: &[u8]) -> Result<(Vec<f32>, u32, u16), Error> {
  let (fmt, data) = scan_chunks(bytes)?;
  let fmt = fmt.ok_or(Error::NotWav)?;
  let data = data.ok_or(Error::NoAudio)?;
  if fmt.len() < 16 {
    return Err(Error::UnsupportedFormat);
  }
  let format = u16::from_le_bytes([fmt[0], fmt[1]]);
  let channels = u16::from_le_bytes([fmt[2], fmt[3]]).max(1);
  let rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
  let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
  if rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  let samples = match format {
    1 if matches!(bits, 8 | 16 | 24 | 32) => decode_pcm_raw(data, bits, usize::from(channels)),
    3 if matches!(bits, 32 | 64) => decode_float_raw(data, bits, usize::from(channels)),
    _ => return Err(Error::UnsupportedFormat),
  };
  if samples.is_empty() {
    return Err(Error::NoAudio);
  }
  Ok((samples, rate, channels))
}

/// PCM → 交错样本（每声道一个值，顺序 = 帧 × 声道，不混音）。
fn decode_pcm_raw(data: &[u8], bits: u16, channels: usize) -> Vec<f32> {
  let bps = (bits / 8).max(1) as usize;
  if bps > 4 || data.len() < bps * channels {
    return Vec::new();
  }
  let frames = data.len() / (bps * channels);
  let scale = match bits {
    8 => 128.0,
    _ => 2f32.powi(bits as i32 - 1),
  };
  let unsigned = bits == 8;
  let mut out = Vec::with_capacity(frames * channels);
  for f in 0..frames {
    for c in 0..channels {
      let i = (f * channels + c) * bps;
      let raw = if unsigned {
        (data[i] as f32) - 128.0
      } else {
        read_i(data, i, bps)
      };
      out.push(raw / scale);
    }
  }
  out
}

/// 浮点 PCM → 交错样本（不混音）。
fn decode_float_raw(data: &[u8], bits: u16, channels: usize) -> Vec<f32> {
  let bps = match bits {
    32 => 4usize,
    64 => 8usize,
    _ => return Vec::new(),
  };
  if data.len() < bps * channels {
    return Vec::new();
  }
  let frames = data.len() / (bps * channels);
  let mut out = Vec::with_capacity(frames * channels);
  // NaN / Inf 样本原样透传（不清洗）：下游 Goertzel / 解调对它们的传播最终表现为
  // 「无结果」而非崩溃。严格归一会让极值裁剪下的合法样本失真，得不偿失。
  for f in 0..frames {
    for c in 0..channels {
      let i = (f * channels + c) * bps;
      out.push(if bits == 32 {
        f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
      } else {
        f64::from_le_bytes([
          data[i],
          data[i + 1],
          data[i + 2],
          data[i + 3],
          data[i + 4],
          data[i + 5],
          data[i + 6],
          data[i + 7],
        ]) as f32
      });
    }
  }
  out
}

/// 字节序读取小端有符号整型（8/16/24/32 位）。
fn read_i(b: &[u8], i: usize, bytes_per_sample: usize) -> f32 {
  let v = match bytes_per_sample {
    1 => i8::from_le_bytes([b[i]]) as i32,
    2 => i16::from_le_bytes([b[i], b[i + 1]]) as i32,
    3 => {
      let x = i32::from_le_bytes([b[i], b[i + 1], b[i + 2], 0]);
      // 24 位符号扩展
      (x << 8) >> 8
    }
    _ => i32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]),
  };
  v as f32
}

fn decode_pcm(data: &[u8], bits: u16, channels: usize) -> Vec<f32> {
  let bps = (bits / 8).max(1) as usize;
  if bps > 4 || data.len() < bps * channels {
    return Vec::new();
  }
  let frames = data.len() / (bps * channels);
  let scale = match bits {
    8 => 128.0, // 无符号 8 位，偏置 128
    _ => 2f32.powi(bits as i32 - 1),
  };
  let unsigned = bits == 8;

  let mut out = Vec::with_capacity(frames);
  for f in 0..frames {
    let mut acc = 0.0f32;
    for c in 0..channels {
      let i = (f * channels + c) * bps;
      let raw = if unsigned {
        (data[i] as f32) - 128.0
      } else {
        read_i(data, i, bps)
      };
      acc += raw / scale;
    }
    out.push(acc / channels as f32);
  }
  out
}

fn decode_float(data: &[u8], bits: u16, channels: usize) -> Vec<f32> {
  let bps = match bits {
    32 => 4usize,
    64 => 8usize,
    _ => return Vec::new(),
  };
  if data.len() < bps * channels {
    return Vec::new();
  }
  let frames = data.len() / (bps * channels);
  let mut out = Vec::with_capacity(frames);
  for f in 0..frames {
    let mut acc = 0.0f32;
    for c in 0..channels {
      let i = (f * channels + c) * bps;
      let v = if bits == 32 {
        f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]])
      } else {
        f64::from_le_bytes([
          data[i],
          data[i + 1],
          data[i + 2],
          data[i + 3],
          data[i + 4],
          data[i + 5],
          data[i + 6],
          data[i + 7],
        ]) as f32
      };
      acc += v;
    }
    out.push(acc / channels as f32);
  }
  out
}

#[cfg(test)]
mod interleave_tests {
  use super::*;

  #[test]
  fn interleaved_keeps_channels_separate() {
    // 立体声：左声道恒 1、右声道恒 -1（16 位）→ 交错解析后交替出现，
    // 而 parse() 的平均会得到全 0 —— 这正是多声道信号不能用混音解析的原因。
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36u32 + 160).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&8000u32.to_le_bytes());
    wav.extend_from_slice(&32000u32.to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&160u32.to_le_bytes());
    for _ in 0..40 {
      wav.extend_from_slice(&16384i16.to_le_bytes());
      wav.extend_from_slice(&(-16384i16).to_le_bytes());
    }
    let (samples, rate, channels) = parse_interleaved(&wav).expect("解析");
    assert_eq!(rate, 8000);
    assert_eq!(channels, 2);
    assert_eq!(samples.len(), 80);
    assert!(samples.iter().step_by(2).all(|&s| s > 0.0), "左声道为正");
    assert!(
      samples.iter().skip(1).step_by(2).all(|&s| s < 0.0),
      "右声道为负"
    );
    // 对照：混音版全 0。
    let (mono, _) = parse(&wav).expect("解析");
    assert!(mono.iter().all(|&s| s.abs() < 1e-3), "混音后左右抵消");
  }

  /// 块头（id + 声明长度）。
  fn chunk(id: &[u8; 4], size: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(id);
    out.extend_from_slice(&size.to_le_bytes());
    out
  }

  /// RIFF/WAVE 外壳。
  fn riff(body: &[u8]) -> Vec<u8> {
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(body.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(body);
    wav
  }

  #[test]
  fn chunk_size_overflow_is_rejected_without_panicking() {
    // 声明长度 0xFFFFFFF0：在 wasm32（usize = 32 位）上 `body + size` 会回绕，
    // 旧实现让「超出文件」的比较失效，紧接着的切片直接 panic。
    let mut body = chunk(b"fmt ", 0xFFFF_FFF0);
    body.extend_from_slice(&[0u8; 8]);
    let wav = riff(&body);
    assert_eq!(parse(&wav), Err(Error::Truncated));
    assert_eq!(parse_interleaved(&wav), Err(Error::Truncated));
  }

  #[test]
  fn overflowing_data_chunk_falls_back_to_readable_bytes() {
    // data 块声明长度溢出（或补长）时按实际可读范围夹取，而不是 panic。
    let mut body = chunk(b"fmt ", 16);
    body.extend_from_slice(&[1, 0, 1, 0]); // PCM / 单声道
    body.extend_from_slice(&8000u32.to_le_bytes());
    body.extend_from_slice(&16000u32.to_le_bytes());
    body.extend_from_slice(&[2, 0]); // 块对齐
    body.extend_from_slice(&[16, 0]); // 16 位
    body.extend_from_slice(&chunk(b"data", 0xFFFF_FFF0));
    body.extend_from_slice(&16384i16.to_le_bytes());
    body.extend_from_slice(&(-16384i16).to_le_bytes());
    let wav = riff(&body);
    let (samples, rate) = parse(&wav).expect("容错解析");
    assert_eq!(rate, 8000);
    assert_eq!(samples.len(), 2);
    let (interleaved, _, channels) = parse_interleaved(&wav).expect("容错解析");
    assert_eq!(channels, 1);
    assert_eq!(interleaved.len(), 2);
  }
}
