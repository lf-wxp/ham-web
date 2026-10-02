//! 行组装与图像重建：从字速率包络与行起点提取 909×N 的双通道灰度图。
//!
//! 每通道（1040 字）布局：sync 39 + space 47 + 图像 909 + 遥测 45。
//! 视频为倒置 AM（幅度越大越黑），借助每行遥测楔形（黑→白斜坡）的黑/白端点做校准。

use super::{
  CHANNEL_WORDS, Error, IMAGE_WORDS, LINE_WORDS, SPACE_WORDS, SYNC_WORDS, TELEMETRY_WORDS,
  WORD_RATE,
};

/// 通道 A 图像区起始（相对行起点）。
const CH_A_IMG: usize = SYNC_WORDS + SPACE_WORDS;
/// 通道 B 图像区起始（相对行起点）。
const CH_B_IMG: usize = CHANNEL_WORDS + SYNC_WORDS + SPACE_WORDS;

/// 解码出的 APT 图像（双通道灰度）。
#[derive(Debug, Clone, PartialEq)]
pub struct AptImage {
  /// 每通道宽度（= 909 像素）。
  pub width: u32,
  /// 高度（行数）。
  pub height: u32,
  /// 通道 A 灰度像素（高度 × 宽度，行优先）。
  pub channel_a: Vec<u8>,
  /// 通道 B 灰度像素。
  pub channel_b: Vec<u8>,
  /// 原始音频采样率。
  pub source_sample_rate: u32,
  /// 音频时长（秒）。
  pub duration_seconds: f64,
  /// 检测到的行数。
  pub lines: usize,
}

impl AptImage {
  /// 由通道 A（可见光）与通道 B（红外）合成假彩色 RGBA 像素（高度 × 宽度 × 4）。
  ///
  /// 采用线性合成：红 = 通道 A、绿 = 通道 B、蓝 = 通道 B。
  /// 陆表/植被在红外通道更亮而偏青绿，云（双通道均亮）偏白，海面（均暗）偏黑。
  #[must_use]
  pub fn false_color(&self) -> Vec<u8> {
    let mut out = vec![0u8; self.channel_a.len() * 4];
    for (i, (&a, &b)) in self.channel_a.iter().zip(&self.channel_b).enumerate() {
      let o = i * 4;
      out[o] = a;
      out[o + 1] = b;
      out[o + 2] = b;
      out[o + 3] = 255;
    }
    out
  }
}

/// 从字速率包络与行起点组装图像。
pub fn assemble(words: &[f32], lines: &[usize], source_rate: u32) -> Result<AptImage, Error> {
  if lines.len() < 2 {
    return Err(Error::NoAudio);
  }
  // 先确定有效行数：丢弃超出 words 范围的尾部行（例如同步检测在末尾多锁的一行）。
  let rows = lines
    .iter()
    .take_while(|&&start| start + LINE_WORDS <= words.len())
    .count();
  if rows < 2 {
    return Err(Error::NoAudio);
  }

  let width = IMAGE_WORDS as u32;
  let height = rows as u32;
  let mut a = vec![0u8; IMAGE_WORDS * rows];
  let mut b = vec![0u8; IMAGE_WORDS * rows];

  for (row, &start) in lines.iter().take(rows).enumerate() {
    let ch_a = &words[start + CH_A_IMG..start + CH_A_IMG + IMAGE_WORDS];
    let ch_b = &words[start + CH_B_IMG..start + CH_B_IMG + IMAGE_WORDS];
    let tel_a = &words[start + CHANNEL_WORDS - TELEMETRY_WORDS..start + CHANNEL_WORDS];
    let tel_b = &words[start + LINE_WORDS - TELEMETRY_WORDS..start + LINE_WORDS];

    let (black_a, white_a) = calibrate(ch_a, tel_a);
    let (black_b, white_b) = calibrate(ch_b, tel_b);

    for (col, (&va, &vb)) in ch_a.iter().zip(ch_b).enumerate() {
      let idx = row * IMAGE_WORDS + col;
      a[idx] = to_gray(va, black_a, white_a);
      b[idx] = to_gray(vb, black_b, white_b);
    }
  }

  Ok(AptImage {
    width,
    height,
    channel_a: a,
    channel_b: b,
    source_sample_rate: source_rate,
    duration_seconds: words.len() as f64 / WORD_RATE as f64,
    lines: rows,
  })
}

/// 由遥测楔形给出黑/白参考（倒置视频：高幅度=黑，低幅度=白）。
///
/// 返回 `(black, white)`；遥测退化时回退到图像区 min/max。
fn calibrate(img: &[f32], tel: &[f32]) -> (f32, f32) {
  let (white, black) = percentile_ends(tel);
  if black - white < 1e-4 {
    let (lo, hi) = min_max(img);
    if hi - lo < 1e-6 {
      return (1.0, 0.0);
    }
    return (hi, lo);
  }
  (black, white)
}

/// 取遥测段排序后约 5% / 95% 分位作为白/黑端点（抗毛刺）。
fn percentile_ends(tel: &[f32]) -> (f32, f32) {
  if tel.len() < 4 {
    return min_max(tel);
  }
  let mut v = tel.to_vec();
  v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  let lo = v.len() * 5 / 100;
  let hi = v.len() * 95 / 100;
  (v[lo], v[hi])
}

fn min_max(xs: &[f32]) -> (f32, f32) {
  let mut mn = f32::INFINITY;
  let mut mx = f32::NEG_INFINITY;
  for &x in xs {
    if x < mn {
      mn = x;
    }
    if x > mx {
      mx = x;
    }
  }
  if mn.is_finite() { (mn, mx) } else { (0.0, 1.0) }
}

/// 幅度 → 灰度（0 黑，255 白）。
fn to_gray(v: f32, black: f32, white: f32) -> u8 {
  let span = black - white;
  if span <= 1e-6 {
    return 128;
  }
  let t = ((black - v) / span).clamp(0.0, 1.0);
  (t * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gray_maps_inverted_video() {
    // 黑参考 = 0.9（高幅度），白参考 = 0.1（低幅度）
    assert_eq!(to_gray(0.9, 0.9, 0.1), 0); // 黑
    assert_eq!(to_gray(0.1, 0.9, 0.1), 255); // 白
    assert_eq!(to_gray(0.5, 0.9, 0.1), 128); // 中灰
  }

  #[test]
  fn assemble_builds_expected_dimensions() {
    let mut words = vec![0.5f32; LINE_WORDS * 3];
    // 遥测楔形：黑→白斜坡（每行通道 A/B）
    for line in 0..3 {
      let base = line * LINE_WORDS;
      for k in 0..TELEMETRY_WORDS {
        let v = 0.9 - 0.8 * (k as f32 / TELEMETRY_WORDS as f32);
        words[base + SYNC_WORDS + SPACE_WORDS + IMAGE_WORDS + k] = v; // tel A
        words[base + CHANNEL_WORDS + SYNC_WORDS + SPACE_WORDS + IMAGE_WORDS + k] = v; // tel B
      }
    }
    let lines = vec![0, LINE_WORDS, 2 * LINE_WORDS];
    let img = assemble(&words, &lines, 20_800).unwrap();
    assert_eq!(img.width, IMAGE_WORDS as u32);
    assert_eq!(img.height, 3);
    assert_eq!(img.lines, 3);
    assert_eq!(img.channel_a.len(), IMAGE_WORDS * 3);
  }

  #[test]
  fn assemble_drops_trailing_partial_line() {
    // words 只够 2 行，但 lines 给了 3 个起点，最后一行超出范围应被丢弃，
    // 且像素缓冲区长度必须与 width×height 一致（回归：ImageData 构造失败）。
    let mut words = vec![0.5f32; LINE_WORDS * 2 + 10];
    for line in 0..2 {
      let base = line * LINE_WORDS;
      for k in 0..TELEMETRY_WORDS {
        let v = 0.9 - 0.8 * (k as f32 / TELEMETRY_WORDS as f32);
        words[base + SYNC_WORDS + SPACE_WORDS + IMAGE_WORDS + k] = v;
        words[base + CHANNEL_WORDS + SYNC_WORDS + SPACE_WORDS + IMAGE_WORDS + k] = v;
      }
    }
    let lines = vec![0, LINE_WORDS, 2 * LINE_WORDS];
    let img = assemble(&words, &lines, 20_800).unwrap();
    assert_eq!(img.height, 2);
    assert_eq!(img.lines, 2);
    assert_eq!(img.channel_a.len(), IMAGE_WORDS * 2);
    assert_eq!(img.channel_b.len(), IMAGE_WORDS * 2);
    assert_eq!(
      img.channel_a.len(),
      (img.width as usize) * (img.height as usize)
    );
  }

  #[test]
  fn false_color_composes_channels() {
    let img = AptImage {
      width: 1,
      height: 2,
      channel_a: vec![200, 10],
      channel_b: vec![50, 220],
      source_sample_rate: 20_800,
      duration_seconds: 0.5,
      lines: 2,
    };
    let rgba = img.false_color();
    assert_eq!(&rgba[0..4], &[200, 50, 50, 255]);
    assert_eq!(&rgba[4..8], &[10, 220, 220, 255]);
  }
}
