//! WAV（RIFF）解析：支持 PCM 与 IEEE float，单/双声道，8/16/24/32 位。
//!
//! 输出归一化到 [-1, 1] 的单声道样本（多声道取平均）与原始采样率。

use super::Error;

/// 解析 WAV 字节，返回 `(归一化单声道样本, 采样率)`。
pub fn parse(bytes: &[u8]) -> Result<(Vec<f32>, u32), Error> {
  if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    return Err(Error::NotWav);
  }

  let mut fmt: Option<&[u8]> = None;
  let mut data: Option<&[u8]> = None;
  let mut pos = 12usize;

  while pos + 8 <= bytes.len() {
    let id = &bytes[pos..pos + 4];
    let size = u32::from_le_bytes([
      bytes[pos + 4],
      bytes[pos + 5],
      bytes[pos + 6],
      bytes[pos + 7],
    ]) as usize;
    let body = pos + 8;
    if body + size > bytes.len() {
      // 部分文件在 data 块后有补齐或截断，容错到实际可读范围。
      if id == b"data" {
        data = Some(&bytes[body..bytes.len().min(body + size)]);
        break;
      }
      return Err(Error::Truncated);
    }
    match id {
      b"fmt " => fmt = Some(&bytes[body..body + size]),
      b"data" => {
        data = Some(&bytes[body..body + size]);
        break;
      }
      _ => {}
    }
    // 奇数大小的块会补齐 1 字节对齐。
    pos = body + size + (size & 1);
  }

  let fmt = fmt.ok_or(Error::NotWav)?;
  let data = data.ok_or(Error::NoAudio)?;
  if fmt.len() < 16 {
    return Err(Error::UnsupportedFormat);
  }

  let format = u16::from_le_bytes([fmt[0], fmt[1]]);
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
mod tests {
  use super::*;

  /// 在内存中构造一个 16 位 PCM 单声道 WAV。
  fn make_wav(rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
      out.extend_from_slice(&s.to_le_bytes());
    }
    out
  }

  #[test]
  fn parses_pcm16_mono() {
    let wav = make_wav(8000, &[0, 16384, -16384, 0]);
    let (samples, rate) = parse(&wav).unwrap();
    assert_eq!(rate, 8000);
    assert_eq!(samples.len(), 4);
    assert!((samples[1] - 0.5).abs() < 1e-6);
    assert!((samples[2] + 0.5).abs() < 1e-6);
  }

  #[test]
  fn parses_pcm8_unsigned() {
    let data = [128u8, 255, 0];
    let mut wav = vec![b'R', b'I', b'F', b'F'];
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&8000u32.to_le_bytes());
    wav.extend_from_slice(&8000u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&8u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);

    let (samples, _) = parse(&wav).unwrap();
    assert_eq!(samples.len(), 3);
    assert!(samples[0].abs() < 1e-6);
    assert!(samples[1] > 0.99, "255 应接近 +1，得到 {}", samples[1]);
    assert!(samples[2] < -0.99, "0 应接近 -1，得到 {}", samples[2]);
  }

  #[test]
  fn rejects_non_wav() {
    assert_eq!(parse(b"hello world"), Err(Error::NotWav));
    assert_eq!(parse(&[]), Err(Error::NotWav));
  }
}
