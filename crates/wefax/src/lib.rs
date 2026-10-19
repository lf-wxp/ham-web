//! WEFAX（WMO 短波气象传真）解码：AM 视频信号 → 灰度图。
//!
//! 纯 Rust、无外部依赖（除 WAV 解析），可同时用于 WASM 前端与命令行 / 测试。
//!
//! # 协议速览
//!
//! WEFAX 是图像传真：每行以一个同步脉冲开头，接着是像素，亮度逐像素变化。本实现采用的
//! 简化模型是**幅度 = 亮度**（黑 = 高幅度、白 = 低幅度），基带音频直接就是视频信号。
//!
//! # 诚实说明
//!
//! 这个模型**不等于**电台里收到的真实 HF 传真。据我们所知，短波气象传真走 SSB 时，
//! 音频是 1500 Hz（黑）～ 2300 Hz（白）的**调频副载波**（中心约 1900 Hz），而不是原始
//! 幅度；卫星 WEFAX（如 GOES）则是 2400 Hz 副载波上的调幅。本实现两者都**没有解调**：
//! 拿真实电台的录音来解，得到的只会是噪声图。要解真实信号，需先对 1500/2300 Hz 做 FM
//! 解调再进入下面的行同步，并按 WMO 规范校准同步时序与像素率。
//!
//! 行速率 / 同步脉冲占比 / 每行像素数同样是**自定口径**（120 LPM、同步占 5%、每行 380
//! 像素），未对照真实 WMO 传真样本校准——往返合成对拍只证明「本实现自洽」。解调假设
//! **行对齐**（首个同步脉冲后按固定行长切分）。真实录音的验收状态见
//! `docs/hardware-verification.md` 的 E4。

/// 工作采样率（Hz）。
pub const SAMPLE_RATE: u32 = 8_000;
/// 行速率（120 LPM = 2 行/秒）。
pub const LINE_RATE: f64 = 2.0;
/// 同步脉冲占行长的比例。
pub const SYNC_FRACTION: f64 = 0.05;
/// 每行像素数。
pub const PIXELS_PER_LINE: usize = 380;
/// 每行采样数（0.5 秒）。
pub const LINE_SAMPLES: usize = (SAMPLE_RATE as f64 / LINE_RATE) as usize;
/// 同步脉冲采样数。
pub const SYNC_SAMPLES: usize = (LINE_SAMPLES as f64 * SYNC_FRACTION) as usize;
/// 每像素采样数。
pub const PIXEL_SAMPLES: usize = (LINE_SAMPLES - SYNC_SAMPLES) / PIXELS_PER_LINE;
/// 每行「同步 + 像素」必须放得进行长：像素区按整数采样切分，余下的零头丢掉。
///
/// 这是解码端 `&working[off..off + PIXEL_SAMPLES]` 不越界的**唯一**依据（行数按
/// `len / LINE_SAMPLES` 取整，最后一行最后一个像素正好落在行尾）。改这几个常量时在这里
/// 编译期炸掉，而不是在 wasm 里变成无信息的 `worker-run-failed:unreachable`。
const _: () = assert!(SYNC_SAMPLES + PIXELS_PER_LINE * PIXEL_SAMPLES <= LINE_SAMPLES);

/// 行同步判定用的相对电平：取样本峰值的这个比例。
///
/// 不能写死绝对值（如 `> 0.9`）：录音留 headroom、经 `ham_web_dsp` 衰减、或 AM 检波后
/// 黑电平只有 0.5 时，同步脉冲再干净也过不了门限，整段被判成「无信号」。按峰值取相对
/// 门限与 [`crate::BLACK`] 的定义（黑 = 该路信号的高电平）一致。
pub const SYNC_LEVEL_RATIO: f32 = 0.6;

/// 黑（高幅度）。
pub const BLACK: f32 = 1.0;
/// 白（低幅度，本实现取 0；真实 WEFAX 约 5% 白底未建模）。
pub const WHITE: f32 = 0.0;

use std::borrow::Cow;

/// 重建的灰度图（`gray` 一字节一像素，0 = 白、255 = 黑）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WefaxImage {
  pub width: usize,
  pub height: usize,
  pub gray: Vec<u8>,
}

/// 解码错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
  NotWav,
  UnsupportedFormat,
  Truncated,
  NoAudio,
  TooShort,
  /// 音频足够长，但找不到行同步脉冲（噪声、别的模式、或不是传真）。
  NoSync,
  /// `WefaxImage` 的 `width × height` 与 `gray` 长度不符（`modulate` 的入参校验）。
  InvalidImage,
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let msg = match self {
      Self::NotWav => "不是有效的 WAV 文件",
      Self::UnsupportedFormat => "不支持的音频格式（仅支持 PCM / IEEE float WAV）",
      Self::Truncated => "音频文件数据不完整",
      Self::NoAudio => "文件中没有可解码的音频数据",
      Self::TooShort => "音频过短，无法解码出图像",
      Self::NoSync => "音频里找不到行同步脉冲",
      Self::InvalidImage => "灰度图尺寸与数据长度不符",
    };
    f.write_str(msg)
  }
}

impl Error {
  #[must_use]
  pub fn code(&self) -> &'static str {
    match self {
      Self::NotWav => "not-wav",
      Self::UnsupportedFormat => "unsupported-format",
      Self::Truncated => "truncated",
      Self::NoAudio => "no-audio",
      Self::TooShort => "too-short",
      // 找不到行同步与其它解码器的「没有可解调的信号」同一口径；worker 侧不再单独改写。
      Self::NoSync => "no-signal:WEFAX",
      // 不复用 LRPT 的 `image-incomplete:<帧数>`：那个码带占位符，走到 WEFAX 会显示
      // 「图像不完整（）」。这里只在 `modulate` 的入参校验出现，用独立码更诚实。
      Self::InvalidImage => "image-size-mismatch",
    }
  }
}

impl std::error::Error for Error {}

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

/// 灰度图 → AM 视频采样（每行：同步脉冲 + 像素；只做 8000 Hz，供合成对拍）。
///
/// 尺寸不符时返回 [`Error::InvalidImage`]：`gray` 比 `width × height` 短会越界 panic，
/// 而 `width` 不等于 [`PIXELS_PER_LINE`] 会生成解码端按固定行长切不开的音频。
pub fn modulate(image: &WefaxImage) -> Result<Vec<f32>, Error> {
  if image.width != PIXELS_PER_LINE || image.gray.len() != image.width.saturating_mul(image.height)
  {
    return Err(Error::InvalidImage);
  }
  let mut out = Vec::with_capacity(image.height * LINE_SAMPLES);
  for row in 0..image.height {
    out.extend(std::iter::repeat_n(BLACK, SYNC_SAMPLES));
    for col in 0..image.width {
      let amp = image.gray[row * image.width + col] as f32 / 255.0;
      out.extend(std::iter::repeat_n(amp, PIXEL_SAMPLES));
    }
  }
  Ok(out)
}

/// 解码一段 WAV 音频字节，返回重建的灰度图。
pub fn decode(bytes: &[u8]) -> Result<WefaxImage, Error> {
  let (samples, rate) = ham_web_wav::parse(bytes)?;
  decode_samples(&samples, rate)
}

/// 从归一化单声道样本（[-1, 1]）与采样率解码。
pub fn decode_samples(samples: &[f32], sample_rate: u32) -> Result<WefaxImage, Error> {
  if sample_rate == 0 {
    return Err(Error::UnsupportedFormat);
  }
  // 采样率已经是工作采样率时不用拷贝：WAV 常有几 MB，`to_vec` 是一份纯浪费。
  let working: Cow<'_, [f32]> = if sample_rate == SAMPLE_RATE {
    Cow::Borrowed(samples)
  } else {
    Cow::Owned(ham_web_wav::resample(samples, sample_rate, SAMPLE_RATE))
  };
  let working = working.as_ref();
  if working.len() < LINE_SAMPLES {
    return Err(Error::TooShort);
  }
  // 同步门限按整段峰值取相对值：绝对电平会把留了 headroom 的录音整段判成无信号。
  let level = sync_level(working);
  // 找不到同步脉冲不等于「音频过短」：一段足够长的噪声也走这条路，文案要分开
  // （前者换录音，后者只需录长一点）。
  let Some(sync_start) = find_trusted_sync(working, level) else {
    return Err(Error::NoSync);
  };
  let lines = (working.len() - sync_start) / LINE_SAMPLES;
  if lines == 0 {
    // 同步脉冲落在最后半秒里：只剩脉冲没有行，等同于解不出图。
    return Err(Error::TooShort);
  }
  let mut gray = Vec::with_capacity(lines * PIXELS_PER_LINE);
  for line in 0..lines {
    let base = sync_start + line * LINE_SAMPLES + SYNC_SAMPLES;
    for p in 0..PIXELS_PER_LINE {
      let off = base + p * PIXEL_SAMPLES;
      let amp = mean(&working[off..off + PIXEL_SAMPLES]);
      gray.push((amp.clamp(0.0, 1.0) * 255.0).round() as u8);
    }
  }
  Ok(WefaxImage {
    width: PIXELS_PER_LINE,
    height: lines,
    gray,
  })
}

/// 同步判定门限：整段峰值 × [`SYNC_LEVEL_RATIO`]。
///
/// `f32::max` 会忽略 NaN，所以脏样本不会把峰值抬成 NaN；全静音时门限为 0，`s > 0`
/// 对恒零样本不成立，自然落到 `NoSync`。
fn sync_level(samples: &[f32]) -> f32 {
  let peak = samples.iter().copied().fold(0.0_f32, f32::max);
  peak * SYNC_LEVEL_RATIO
}

/// 找第一个同步脉冲（持续高于 `level` 的段）的起始采样。
fn find_sync(samples: &[f32], level: f32) -> Option<usize> {
  let mut run = 0usize;
  for (i, &s) in samples.iter().enumerate() {
    if s > level {
      run += 1;
      if run >= SYNC_SAMPLES {
        return Some(i + 1 - SYNC_SAMPLES);
      }
    } else {
      run = 0;
    }
  }
  None
}

/// 找第一个**可信**的同步起点：候选之后还有同步脉冲才能采信。
///
/// 图像本身也有全黑的行或色块，单看「一段持续高电平」会把它们当成行同步，整幅图从此
/// 错位（解出来是一堆斜纹而不是报错）。传真每行都以同步开头，所以拿「往后一到两行
/// 是否同样以同步开头」做校验；录音太短、没有一整行可校验时按可信处理。
fn find_trusted_sync(samples: &[f32], level: f32) -> Option<usize> {
  let mut from = 0usize;
  loop {
    let rest = samples.get(from..)?;
    let offset = find_sync(rest, level)?;
    let start = from + offset;
    if repeats_each_line(samples, start, level) {
      return Some(start);
    }
    // 跳过这段黑电平：同一个黑块里的其它起点同样不可信，逐个重试只是白扫。
    from = start + SYNC_SAMPLES;
  }
}

/// `start` 之后的连续两行是否也以同步脉冲开头（可校验的行数不足时按通过处理）。
fn repeats_each_line(samples: &[f32], start: usize, level: f32) -> bool {
  for lines in [1usize, 2] {
    let Some(at) = start.checked_add(lines * LINE_SAMPLES) else {
      return false;
    };
    let Some(end) = at.checked_add(SYNC_SAMPLES) else {
      return false;
    };
    // 录音在下一行之前就结束了：没有可校验的证据，不能因此把候选判死。
    if end > samples.len() {
      return true;
    }
    if !samples[at..end].iter().all(|&s| s > level) {
      return false;
    }
  }
  true
}

fn mean(seg: &[f32]) -> f32 {
  seg.iter().sum::<f32>() / seg.len() as f32
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pattern() -> WefaxImage {
    // 380×8 的简单图案：上半白、下半黑，加一条灰渐变。
    let mut gray = Vec::with_capacity(PIXELS_PER_LINE * 8);
    for row in 0..8 {
      for col in 0..PIXELS_PER_LINE {
        let v = if row < 4 {
          0
        } else if row < 6 {
          255
        } else {
          ((col as f32 / PIXELS_PER_LINE as f32) * 255.0) as u8
        };
        gray.push(v);
      }
    }
    WefaxImage {
      width: PIXELS_PER_LINE,
      height: 8,
      gray,
    }
  }

  #[test]
  fn round_trips_an_image() {
    let img = pattern();
    let samples = modulate(&img).expect("尺寸一致");
    let decoded = decode_samples(&samples, SAMPLE_RATE).unwrap();
    assert_eq!(decoded.width, img.width);
    assert_eq!(decoded.height, img.height);
    assert_eq!(decoded.gray, img.gray);
  }

  #[test]
  fn quiet_recordings_still_find_the_sync() {
    // 留 headroom 的录音（峰值只有满幅的 45%）不能因为「黑电平 < 0.9」被判成无信号。
    // 本实现的口径是「幅度 = 亮度」，没有 AGC，所以不比对灰度值，只要求解出行同步。
    let img = pattern();
    let samples: Vec<f32> = modulate(&img)
      .expect("尺寸一致")
      .iter()
      .map(|s| s * 0.45)
      .collect();
    let decoded = decode_samples(&samples, SAMPLE_RATE).expect("半幅录音也应解得出");
    assert_eq!(decoded.width, img.width);
    assert_eq!(decoded.height, img.height);
  }

  #[test]
  fn handles_leading_silence() {
    let img = pattern();
    let samples = modulate(&img).expect("尺寸一致");
    let mut shifted = vec![0.0f32; 400];
    shifted.extend_from_slice(&samples);
    let decoded = decode_samples(&shifted, SAMPLE_RATE).unwrap();
    assert_eq!(decoded.gray, img.gray);
  }

  #[test]
  fn resamples_other_rates() {
    let img = pattern();
    let samples = modulate(&img).expect("尺寸一致");
    let up = ham_web_wav::resample(&samples, SAMPLE_RATE, 16_000);
    let decoded = decode_samples(&up, 16_000).unwrap();
    // 重采样往返允许每个像素 ±1 的量化误差。
    assert_eq!(decoded.gray.len(), img.gray.len());
    for (a, b) in decoded.gray.iter().zip(&img.gray) {
      assert!((*a as i16 - *b as i16).abs() <= 1, "{a} vs {b}");
    }
  }

  #[test]
  fn a_black_burst_before_the_message_is_not_taken_as_sync() {
    // 图像本身有全黑行/色块：单看「一段持续高电平」会把黑突发当成行同步，整幅错位。
    // 同步脉冲每行都有，用「往后两行是否同样以同步开头」把它挡掉。
    let img = pattern();
    let mut samples = vec![1.0f32; 600];
    samples.extend(vec![0.0f32; 1_500]);
    samples.extend(modulate(&img).expect("尺寸一致"));
    let decoded = decode_samples(&samples, SAMPLE_RATE).unwrap();
    assert_eq!(decoded.gray, img.gray);
  }

  #[test]
  fn long_silence_reports_no_sync_not_too_short() {
    // 长度足够但没有同步脉冲：文案必须与「音频过短」分开（换录音 vs 录长一点）。
    let silence = vec![0.0f32; LINE_SAMPLES * 10];
    assert_eq!(
      decode_samples(&silence, SAMPLE_RATE).err(),
      Some(Error::NoSync)
    );
  }

  #[test]
  fn modulate_rejects_inconsistent_images() {
    let mut short = pattern();
    short.gray.pop();
    assert_eq!(modulate(&short).err(), Some(Error::InvalidImage));

    let wrong_width = WefaxImage {
      width: 100,
      height: 8,
      gray: vec![0; 800],
    };
    assert_eq!(modulate(&wrong_width).err(), Some(Error::InvalidImage));
  }

  #[test]
  fn short_or_silent_audio_is_too_short() {
    assert!(decode_samples(&[], 8000).is_err());
    assert_eq!(decode_samples(&[0.0f32; 100], 8000), Err(Error::TooShort));
    let img = pattern();
    let samples = modulate(&img).expect("尺寸一致");
    assert!(decode_samples(&samples[..LINE_SAMPLES / 2], 8000).is_err());
  }
}
