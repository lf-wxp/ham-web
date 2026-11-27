//! WEFAX（WMO 短波气象传真）解码：1500 / 2300 Hz 调频副载波或 AM 视频 → 灰度图。
//!
//! 纯 Rust、无外部依赖（除 WAV 解析），可同时用于 WASM 前端与命令行 / 测试。
//!
//! # 协议速览
//!
//! WEFAX 是图像传真：每行以一个同步脉冲开头，接着是像素，亮度逐像素变化。本实现采用的
//! 简化模型是**幅度 = 亮度**（黑 = 高幅度、白 = 低幅度），基带音频直接就是视频信号。
//!
//! # 两套格式
//!
//! - **真实传真**（[`fax`]）：连续的 1500 / 2300 Hz 调频副载波，没有逐行同步；开头有起始音与
//!   相位线，图像靠两端各自的时钟自由运行（所以会斜）。按 IOC 576 / 288、60–240 LPM 解，
//!   相位线给出行首、行周期与黑白电平，没有相位线就用自相关与整段分布。**页面默认用这一套。**
//! - **简化模型**（本文件与 [`fm`]）：本站最初自拟的口径——每行 380 像素、行首一个 5% 的同步
//!   脉冲，调幅（幅度 = 亮度）或调频两种载体。真实电台不这么发，只剩合成录音自测用。
//!   [`decode`] 会自己判断是调幅还是调频（[`Modulation`]），判错时用 [`decode_with`] 指定。
//!
//! # 诚实说明
//!
//! 真实传真解码用四段公开的真实录音核对过（见 `samples/README.md`）：相位线、行周期与黑白电平
//! 的实测值符合预期，解出的 DWD 地面预报图能读出文字与等压线、竖线竖直。但**页面只给了录音、
//! 没有给出解码后的图像**，所以没有逐像素对错，只有结构断言加人工目视。没有建模的东西列在
//! [`fax`] 的模块文档里（起始音不触发、LSB、彩色传真、强噪声下的抗干扰）。卫星 WEFAX
//! （2400 Hz 调幅副载波）不在这里。
//!
//! 简化模型的几何（120 LPM、同步占 5%、每行 380 像素）是自定口径，往返合成对拍只证明
//! 「本实现自洽」。它解调假设**行对齐**（首个同步脉冲后按固定行长切分）。

pub mod fax;
pub mod fm;

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

/// 输入的调制方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modulation {
  /// 幅度 = 亮度的基带视频。
  Am,
  /// 1500 / 2300 Hz 调频副载波（真实短波传真）。
  Fm,
}

impl Modulation {
  /// 给协议与页面用的稳定小写名。
  #[must_use]
  pub fn code(self) -> &'static str {
    match self {
      Self::Am => "am",
      Self::Fm => "fm",
    }
  }
}

/// 解码结果：图，加上实际采用的解调方式（自动判断时页面要告诉用户）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
  pub image: WefaxImage,
  pub modulation: Modulation,
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

/// 灰度图 → AM 视频采样（幅度 = 亮度；每行：同步脉冲 + 像素；只做 8000 Hz，供合成对拍）。
///
/// 尺寸不符时返回 [`Error::InvalidImage`]：`gray` 比 `width × height` 短会越界 panic，
/// 而 `width` 不等于 [`PIXELS_PER_LINE`] 会生成解码端按固定行长切不开的音频。
pub fn modulate(image: &WefaxImage) -> Result<Vec<f32>, Error> {
  check_image(image)?;
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

/// 灰度图 → 调频副载波采样（真实短波传真的口径，见 [`fm`]）。
pub fn modulate_fm(image: &WefaxImage) -> Result<Vec<f32>, Error> {
  fm::modulate(image)
}

pub(crate) fn check_image(image: &WefaxImage) -> Result<(), Error> {
  if image.width != PIXELS_PER_LINE || image.gray.len() != image.width.saturating_mul(image.height)
  {
    return Err(Error::InvalidImage);
  }
  Ok(())
}

/// 解码一段 WAV 音频字节，返回重建的灰度图（调制方式自动判断）。
pub fn decode(bytes: &[u8]) -> Result<WefaxImage, Error> {
  decode_with(bytes, None).map(|d| d.image)
}

/// 同 [`decode`]，可指定调制方式（`None` = 自动判断），并回报实际采用的方式。
pub fn decode_with(bytes: &[u8], force: Option<Modulation>) -> Result<Decoded, Error> {
  let (samples, rate) = ham_web_wav::parse(bytes)?;
  decode_samples_with(&samples, rate, force)
}

/// 从归一化单声道样本（[-1, 1]）与采样率解码（调制方式自动判断）。
pub fn decode_samples(samples: &[f32], sample_rate: u32) -> Result<WefaxImage, Error> {
  decode_samples_with(samples, sample_rate, None).map(|d| d.image)
}

/// 同 [`decode_samples`]，可指定调制方式（`None` = 自动判断），并回报实际采用的方式。
pub fn decode_samples_with(
  samples: &[f32],
  sample_rate: u32,
  force: Option<Modulation>,
) -> Result<Decoded, Error> {
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
  let modulation = force.unwrap_or_else(|| {
    if fm::looks_fm(working) {
      Modulation::Fm
    } else {
      Modulation::Am
    }
  });
  let image = match modulation {
    Modulation::Am => decode_am(working)?,
    Modulation::Fm => decode_fm(working)?,
  };
  Ok(Decoded { image, modulation })
}

/// 调幅视频：基带幅度直接是亮度。
fn decode_am(working: &[f32]) -> Result<WefaxImage, Error> {
  // 同步门限按整段峰值取相对值：绝对电平会把留了 headroom 的录音整段判成无信号。
  let level = sync_level(working);
  // 找不到同步脉冲不等于「音频过短」：一段足够长的噪声也走这条路，文案要分开
  // （前者换录音，后者只需录长一点）。
  let Some(sync_start) = find_trusted_sync(working, level, SYNC_SAMPLES) else {
    return Err(Error::NoSync);
  };
  render(working.len(), sync_start, |off| {
    mean(&working[off..off + PIXEL_SAMPLES])
  })
}

/// 调频副载波：逐像素解出频率，折成亮度。
fn decode_fm(working: &[f32]) -> Result<WefaxImage, Error> {
  let track = fm::track(working);
  let level = sync_level(&track);
  // 轨迹按「以 i 起头的一个像素窗」估，同步脉冲只有 `SYNC − PIXEL + 1` 个窗完全落在里面，
  // 两端各 `PIXEL − 1` 个窗跨在边界上、值不可靠：要求的连续长度要留出这个余量。
  let Some(rough) = find_trusted_sync(&track, level, SYNC_SAMPLES - 2 * PIXEL_SAMPLES) else {
    return Err(Error::NoSync);
  };
  // 粗行首最多早 `PIXEL − 1` 个样本，真行首在 `rough..rough + PIXEL` 里；挑「格子与像素对齐」
  // 的那个。范围必须从 `rough` 起、不能再往前：整像素错位（把首像素当同步、把下一行的同步头
  // 当末像素）残差同样为零，并列时取最早的，范围往前多伸一个像素就会取到错的那个。
  let start = fm::best_alignment(working, rough..rough + PIXEL_SAMPLES).unwrap_or(rough);
  let black = fm::measured_black_hz(working, start);
  render(working.len(), start, |off| {
    fm::window_hz(&working[off..off + PIXEL_SAMPLES]).map_or(0.0, |hz| fm::level_at(hz, black))
  })
}

/// 按行首 `sync_start` 切行，`pixel(off)` 给出以 `off` 起头的一个像素窗的亮度。
fn render(
  len: usize,
  sync_start: usize,
  pixel: impl Fn(usize) -> f32,
) -> Result<WefaxImage, Error> {
  let lines = (len - sync_start) / LINE_SAMPLES;
  if lines == 0 {
    // 同步脉冲落在最后半秒里：只剩脉冲没有行，等同于解不出图。
    return Err(Error::TooShort);
  }
  let mut gray = Vec::with_capacity(lines * PIXELS_PER_LINE);
  for line in 0..lines {
    let base = sync_start + line * LINE_SAMPLES + SYNC_SAMPLES;
    for p in 0..PIXELS_PER_LINE {
      let amp = pixel(base + p * PIXEL_SAMPLES);
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
fn find_sync(samples: &[f32], level: f32, min_run: usize) -> Option<usize> {
  let mut run = 0usize;
  for (i, &s) in samples.iter().enumerate() {
    if s > level {
      run += 1;
      if run >= min_run {
        return Some(i + 1 - min_run);
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
fn find_trusted_sync(samples: &[f32], level: f32, min_run: usize) -> Option<usize> {
  let mut from = 0usize;
  loop {
    let rest = samples.get(from..)?;
    let offset = find_sync(rest, level, min_run)?;
    let start = from + offset;
    if repeats_each_line(samples, start, level, min_run) {
      return Some(start);
    }
    // 跳过这段黑电平：同一个黑块里的其它起点同样不可信，逐个重试只是白扫。
    from = start + min_run;
  }
}

/// `start` 之后的连续两行是否也以同步脉冲开头（可校验的行数不足时按通过处理）。
fn repeats_each_line(samples: &[f32], start: usize, level: f32, min_run: usize) -> bool {
  for lines in [1usize, 2] {
    let Some(at) = start.checked_add(lines * LINE_SAMPLES) else {
      return false;
    };
    let Some(end) = at.checked_add(min_run) else {
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

  // ---- 调频副载波 ----

  /// 每行都有硬边：黑白交替、全档灰阶、整行全白 / 全黑，逼出像素之间的串扰。
  fn hard_edges() -> WefaxImage {
    let mut gray = Vec::new();
    for row in 0..6 {
      for col in 0..PIXELS_PER_LINE {
        gray.push(match row {
          0 => (col as u32 * 255 / (PIXELS_PER_LINE as u32 - 1)) as u8, // 灰阶全档
          1 => {
            if col % 2 == 0 {
              0
            } else {
              255
            }
          }
          2 => 255,
          3 => 0,
          4 => ((col * 37 + 11) % 256) as u8, // 伪随机
          _ => {
            if (col / 19) % 2 == 0 {
              255
            } else {
              0
            }
          }
        });
      }
    }
    WefaxImage {
      width: PIXELS_PER_LINE,
      height: 6,
      gray,
    }
  }

  /// 确定性的伪随机噪声（线性同余），不引入依赖。
  fn noise(len: usize, amplitude: f32, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..len)
      .map(|_| {
        state = state
          .wrapping_mul(6364136223846793005)
          .wrapping_add(1442695040888963407);
        ((state >> 33) as f32 / (1u64 << 31) as f32 * 2.0 - 1.0) * amplitude
      })
      .collect()
  }

  fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    a.iter()
      .zip(b)
      .map(|(x, y)| (f64::from(*x) - f64::from(*y)).abs())
      .sum::<f64>()
      / b.len() as f64
  }

  #[test]
  fn fm_round_trips_pixel_for_pixel_including_hard_edges() {
    let img = hard_edges();
    let samples = modulate_fm(&img).expect("尺寸一致");
    let d = decode_samples_with(&samples, SAMPLE_RATE, None).unwrap();
    assert_eq!(d.modulation, Modulation::Fm);
    assert_eq!(d.image, img);
  }

  #[test]
  fn fm_is_independent_of_the_carrier_level() {
    // 线性预测只看波形形状，不看幅度：录音电平差 20 倍也要逐像素一致。
    let img = hard_edges();
    let quiet: Vec<f32> = modulate_fm(&img)
      .unwrap()
      .iter()
      .map(|s| s * 0.05)
      .collect();
    assert_eq!(decode_samples(&quiet, SAMPLE_RATE).unwrap(), img);
  }

  #[test]
  fn fm_survives_a_16_bit_wav_round_trip() {
    let img = hard_edges();
    let wav = ham_web_wav::encode(&modulate_fm(&img).unwrap(), SAMPLE_RATE);
    let d = decode_with(&wav, None).unwrap();
    assert_eq!(d.modulation, Modulation::Fm);
    assert_eq!((d.image.width, d.image.height), (img.width, img.height));
    // 16 位量化噪声（约 -90 dB）远小于一个灰阶的频率分辨（800 Hz / 255），逐像素一致。
    assert_eq!(d.image, img);
  }

  #[test]
  fn fm_finds_the_line_start_at_any_offset() {
    // 行首的粗位置会早几个样本，要靠「格子与像素对齐」微调回来；各种前导长度都得逐像素一致。
    let img = hard_edges();
    let samples = modulate_fm(&img).unwrap();
    for lead in [0usize, 1, 3, 7, 9, 10, 11, 13, 250, 1999] {
      let mut shifted = vec![0.0f32; lead];
      shifted.extend_from_slice(&samples);
      let d = decode_samples(&shifted, SAMPLE_RATE).unwrap();
      assert_eq!(d.gray, img.gray, "前导 {lead} 个样本");
    }
  }

  #[test]
  fn fm_corrects_a_mistuned_receiver() {
    // 接收机调谐差 ±60 Hz：整条副载波平移。黑电平从同步脉冲实测，白按 +800 Hz 推，
    // 所以灰度不受影响。
    let img = hard_edges();
    for shift in [-60.0, -25.0, 25.0, 60.0] {
      let samples = fm::modulate_shifted(&img, shift).unwrap();
      let d = decode_samples(&samples, SAMPLE_RATE).unwrap();
      assert_eq!(d.gray, img.gray, "调谐偏差 {shift} Hz");
    }
  }

  #[test]
  fn fm_resamples_other_rates() {
    // 整数倍重采样（16 / 48 kHz）往返逐像素一致；非整数倍（11.025 / 44.1 kHz）靠线性插值，
    // 硬边会被抹一点（实测平均 0.8–4.6 灰阶、最大 34），这里只防整幅错位或反相。
    let img = hard_edges();
    let samples = modulate_fm(&img).unwrap();
    for rate in [11_025u32, 16_000, 44_100, 48_000] {
      let mut up = ham_web_wav::resample(&samples, SAMPLE_RATE, rate);
      // 重采样后的长度可能差几个样本；补一小段静音，别让最后一行因此被舍掉。
      up.extend(vec![0.0f32; 64]);
      let d = decode_samples_with(&up, rate, None).unwrap();
      assert_eq!(d.modulation, Modulation::Fm, "{rate} Hz");
      assert_eq!(d.image.height, img.height, "{rate} Hz");
      if rate % SAMPLE_RATE == 0 {
        assert_eq!(d.image.gray, img.gray, "{rate} Hz");
      } else {
        let mean = mean_abs_diff(&d.image.gray, &img.gray);
        assert!(mean < 8.0, "{rate} Hz：平均误差 {mean:.1}");
      }
    }
  }

  #[test]
  fn fm_a_black_burst_before_the_message_is_not_taken_as_sync() {
    // 1500 Hz 的纯音（黑）排在正文之前：单看「一段黑」会当成行同步，要靠后续行校验挡掉。
    let img = hard_edges();
    let mut samples = modulate_fm(&WefaxImage {
      width: PIXELS_PER_LINE,
      height: 1,
      gray: vec![255; PIXELS_PER_LINE],
    })
    .unwrap();
    samples.truncate(900);
    samples.extend(vec![0.0f32; 1_300]);
    samples.extend(modulate_fm(&img).unwrap());
    assert_eq!(decode_samples(&samples, SAMPLE_RATE).unwrap(), img);
  }

  #[test]
  fn noise_costs_accuracy_but_not_the_image() {
    // 每个像素只有 10 个样本，线性预测对噪声比平滑法敏感——把这个代价量出来而不是回避。
    // 均匀噪声幅度占载波的 5% / 22%（约 28 / 16 dB 信噪比）时，图还在、行数对，实测平均误差
    // 约 1.2 / 7.5 灰阶（最大 7 / 44）；这里的上限留了一倍多余量。
    let img = hard_edges();
    let clean = modulate_fm(&img).unwrap();
    for (amplitude, mean_limit) in [(0.045f32, 3.0), (0.2, 15.0)] {
      let n = noise(clean.len(), amplitude, 7);
      let mut noisy: Vec<f32> = clean.iter().zip(&n).map(|(a, b)| a + b).collect();
      noisy.extend(vec![0.0f32; 64]);
      let d = decode_samples_with(&noisy, SAMPLE_RATE, None).unwrap();
      assert_eq!(d.modulation, Modulation::Fm, "噪声 {amplitude}");
      assert_eq!(d.image.height, img.height, "噪声 {amplitude}");
      let mean = mean_abs_diff(&d.image.gray, &img.gray);
      assert!(
        mean < mean_limit,
        "噪声 {amplitude}：平均误差 {mean:.1} 灰阶"
      );
    }
  }

  #[test]
  fn auto_detection_tells_the_two_apart() {
    let img = pattern();
    let am = modulate(&img).unwrap();
    let fm_audio = modulate_fm(&img).unwrap();
    assert_eq!(
      decode_samples_with(&am, SAMPLE_RATE, None)
        .unwrap()
        .modulation,
      Modulation::Am
    );
    assert_eq!(
      decode_samples_with(&fm_audio, SAMPLE_RATE, None)
        .unwrap()
        .modulation,
      Modulation::Fm
    );
    assert!(!fm::looks_fm(&vec![0.0f32; 4000]), "静音不是调频");
    // 白噪声的线性预测系数在 0 附近、折成 2 kHz，恰在频带里，所以「像调频」；它落到调频路径后
    // 找不到同步脉冲，照样报 NoSync（见 `fm_noise_and_silence_report_no_sync`）。
  }

  #[test]
  fn forcing_a_modulation_overrides_the_guess() {
    let img = pattern();
    let am = modulate(&img).unwrap();
    // 指定对了：与自动一致。
    let forced = decode_samples_with(&am, SAMPLE_RATE, Some(Modulation::Am)).unwrap();
    assert_eq!(forced.image.gray, img.gray);
    // 指定错了：按指定的走，不会悄悄改回自动判断的结果；调幅音频按调频解不出同步。
    assert_eq!(
      decode_samples_with(&am, SAMPLE_RATE, Some(Modulation::Fm)).err(),
      Some(Error::NoSync)
    );
  }

  #[test]
  fn fm_noise_and_silence_report_no_sync() {
    assert_eq!(
      decode_samples_with(&noise(40_000, 0.5, 11), SAMPLE_RATE, Some(Modulation::Fm)).err(),
      Some(Error::NoSync)
    );
    assert_eq!(
      decode_samples_with(&vec![0.0f32; 40_000], SAMPLE_RATE, Some(Modulation::Fm)).err(),
      Some(Error::NoSync)
    );
  }

  #[test]
  fn fm_modulate_rejects_inconsistent_images() {
    let mut short = pattern();
    short.gray.pop();
    assert_eq!(modulate_fm(&short).err(), Some(Error::InvalidImage));
  }

  #[test]
  fn measured_black_level_is_only_trusted_within_the_tuning_tolerance() {
    // 调谐偏差在容限内：用实测值；超出容限（多半不是这路信号）：退回标称 1500 Hz。
    let img = hard_edges();
    for (shift, expected) in [
      (100.0, 1600.0),
      (-100.0, 1400.0),
      (300.0, 1500.0),
      (-300.0, 1500.0),
    ] {
      let samples = fm::modulate_shifted(&img, shift).unwrap();
      let measured = fm::measured_black_hz(&samples, 0);
      assert!(
        (measured - expected).abs() < 0.5,
        "偏差 {shift} Hz 量成 {measured}"
      );
    }
  }

  #[test]
  fn fm_tone_estimate_matches_the_carrier_frequency() {
    // 估计器本身：1500 / 1900 / 2300 Hz 的纯音在 10 个样本里就能估到 0.01 Hz 内。
    for hz in [1500.0f64, 1900.0, 2300.0] {
      let w: Vec<f32> = (0..PIXEL_SAMPLES)
        .map(|n| (std::f64::consts::TAU * hz * n as f64 / 8000.0 + 0.7).sin() as f32)
        .collect();
      let est = fm::window_hz(&w).unwrap();
      assert!((est - hz).abs() < 0.01, "{hz} Hz 估成 {est}");
    }
    assert!(fm::window_hz(&[0.0; 10]).is_none());
  }
}
