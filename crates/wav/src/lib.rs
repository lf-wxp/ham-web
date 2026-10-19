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

/// 允许解析的采样率区间（Hz）。
///
/// 上限挡住「头部写天文数字」的畸形文件；下限挡住 `resample` 里的比值爆炸——1 Hz 的 3 MB
/// 文件换算到 48 kHz 就是上百 GB 的内存申请。真实录音（8 kHz 起）远在这个区间内。
const MIN_RATE: u32 = 4_000;
const MAX_RATE: u32 = 384_000;

/// 归一化单声道样本（[-1, 1]）→ 16 位 PCM 单声道 WAV 字节。
///
/// 各解码器 `examples/generate_sample.rs` 与 `tools` 的样本生成器此前各抄一份 `make_wav`
/// （签名已漂移：有的传采样率、有的硬编码），统一到这里；输出与 [`parse`] 往返一致。
#[must_use]
pub fn encode(samples: &[f32], rate: u32) -> Vec<u8> {
  let data_len = samples.len() * 2;
  // RIFF 头里的长度字段是 u32。wasm32 的线性内存只有 4 GiB，原生调用方也只处理几 MB 的
  // 样本，因此「放不下」属于调用方的 bug：debug 构建直接断死；release 里长度字段饱和到
  // `u32::MAX`，而不是回绕成一个比实际数据短得多的坏头。
  debug_assert!(
    u32::try_from(36 + data_len).is_ok() && rate.checked_mul(2).is_some(),
    "WAV 头字段放不下：{} 个样本、采样率 {rate}",
    samples.len()
  );
  let data_len_field = u32::try_from(data_len).unwrap_or(u32::MAX);
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&data_len_field.saturating_add(36).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes()); // PCM
  out.extend_from_slice(&1u16.to_le_bytes()); // 单声道
  out.extend_from_slice(&rate.to_le_bytes());
  // 字节率 = rate × 声道 × 2 字节；`saturating` 只为不 panic，正常采样率不会触发。
  out.extend_from_slice(&rate.saturating_mul(2).to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
  out.extend_from_slice(&16u16.to_le_bytes()); // 16 位
  out.extend_from_slice(b"data");
  out.extend_from_slice(&data_len_field.to_le_bytes());
  for &s in samples {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}

/// 线性重采样：把 `from` Hz 的样本换算到 `to` Hz。
///
/// 解码器各自需要一个固定工作采样率（AFSK1200 的 48 kHz、WEFAX 的 8 kHz、PSK31 的……），
/// 此前三个 crate 各抄了一份几乎相同的实现，守卫还不一致（只有一份拒绝 `from == 0`）。
/// `from == 0` 会让比值变成 inf，`as usize` 饱和成 `usize::MAX` 后 `with_capacity` 直接崩；
/// `to == 0` 只会得到空结果，同样没意义——两者都按「无法换算」返回空。
/// [`parse`] 已把文件里的采样率限制在 4 kHz–384 kHz，所以经由它进来的比值有界。
#[must_use]
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
  if from == 0 || to == 0 {
    return Vec::new();
  }
  if from == to || samples.len() < 2 {
    return samples.to_vec();
  }
  let ratio = f64::from(to) / f64::from(from);
  let n = (samples.len() as f64 * ratio).round() as usize;
  let last = samples.len() - 1;
  let mut out = Vec::with_capacity(n);
  for i in 0..n {
    let src = i as f64 / ratio;
    let idx = src as usize;
    let frac = (src - idx as f64) as f32;
    let a = samples[idx.min(last)];
    let b = samples[(idx + 1).min(last)];
    out.push(a + (b - a) * frac);
  }
  out
}

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

  if !(MIN_RATE..=MAX_RATE).contains(&rate) {
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
  if !(MIN_RATE..=MAX_RATE).contains(&rate) {
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

#[cfg(test)]
mod encode_tests {
  use super::*;

  #[test]
  fn out_of_range_sampling_rates_are_rejected() {
    // fmt 块的采样率字段在偏移 24；1 Hz 会让重采样申请上百 GB，4000 以下一律拒绝。
    let samples = vec![0.0f32; 8];
    let mut wav = encode(&samples, 8000);
    wav[24..28].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(parse(&wav), Err(Error::UnsupportedFormat));
    wav[24..28].copy_from_slice(&(MAX_RATE + 1).to_le_bytes());
    assert_eq!(parse(&wav), Err(Error::UnsupportedFormat));
    wav[24..28].copy_from_slice(&48_000u32.to_le_bytes());
    assert!(parse(&wav).is_ok());
  }

  #[test]
  fn resample_scales_length_and_refuses_zero_rates() {
    let samples: Vec<f32> = (0..48_000).map(|i| (i as f32 * 0.01).sin()).collect();
    assert_eq!(resample(&samples, 48_000, 24_000).len(), 24_000);
    assert_eq!(resample(&samples, 24_000, 48_000).len(), 96_000);
    assert_eq!(resample(&samples, 48_000, 48_000), samples);
    // 0 Hz 不能换算：既不能崩（`with_capacity(usize::MAX)`），也不能原样放行。
    assert!(resample(&samples, 0, 48_000).is_empty());
    assert!(resample(&samples, 48_000, 0).is_empty());
  }

  #[test]
  fn resample_keeps_a_ramp_monotonic() {
    let ramp: Vec<f32> = (0..100).map(|i| i as f32).collect();
    let up = resample(&ramp, 8_000, 16_000);
    assert!(up.windows(2).all(|w| w[1] >= w[0]), "线性插值不该出现回头");
    assert!((up[1] - 0.5).abs() < 1e-6, "第二个点应落在 0 与 1 的中点");
  }

  #[test]
  fn encode_round_trips_through_parse() {
    let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
    let wav = encode(&samples, 8000);
    let (decoded, rate) = parse(&wav).unwrap();
    assert_eq!(rate, 8000);
    assert_eq!(decoded.len(), samples.len());
    // 16 位量化往返：误差小于半个 LSB。
    for (a, b) in decoded.iter().zip(&samples) {
      assert!((a - b).abs() < 1.0 / 32767.0, "{a} vs {b}");
    }
  }
}
