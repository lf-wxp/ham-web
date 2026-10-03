//! 行同步 + 采样 + 颜色解码 + 图像组装。

use super::{BLACK_HZ, Error, SYNC_HZ, WHITE_HZ, WORK_RATE, modes::ColorKind, modes::Mode};

/// 解码出的 SSTV 图像。
#[derive(Debug, Clone, PartialEq)]
pub struct SstvImage {
  pub width: u32,
  pub height: u32,
  /// 模式名。
  pub mode: String,
  /// RGBA 像素（height × width × 4，行优先）。
  pub rgba: Vec<u8>,
  pub source_sample_rate: u32,
  pub duration_seconds: f64,
}

/// 从 VIS 结束位置起，逐行锁定同步、采样并组装图像。
pub fn assemble(
  freq: &[f32],
  mode: &Mode,
  vis_end: usize,
  source_rate: u32,
) -> Result<SstvImage, Error> {
  let rate = WORK_RATE as f32;
  let sync_samples = (mode.sync_ms * rate / 1000.0).round().max(1.0) as usize;
  let porch_samples = (mode.porch_ms * rate / 1000.0).round() as usize;
  let per_comp = (mode.pixel_us * rate / 1_000_000.0).round().max(1.0) as usize;

  // 每行扫描样本数：RGB = 3×width，YC = 2×width（Y 全 + 两路色度各半）。
  let comps_per_pixel = match mode.color {
    ColorKind::Rgb => 3usize,
    ColorKind::Yc => 2usize,
  };
  let scan_samples = mode.width * comps_per_pixel * per_comp;
  let line_samples = sync_samples + porch_samples + scan_samples;

  // 锁定第一行同步脉冲。
  let mut line_start = find_sync(freq, vis_end, sync_samples).ok_or(Error::NoVis)?;

  let mut rows = Vec::new();
  while line_start + line_samples <= freq.len() && rows.len() < mode.height {
    rows.push(line_start);
    let next = line_start + line_samples;
    // 在预期位置附近重锁，容忍轻微时钟漂移。
    line_start = relock(freq, next, sync_samples);
  }

  if rows.len() < 2 {
    return Err(Error::TooShort);
  }

  let height = rows.len();
  let mut rgba = vec![0u8; mode.width * height * 4];
  for (row, &start) in rows.iter().enumerate() {
    let scan_start = start + sync_samples + porch_samples;
    match mode.color {
      ColorKind::Rgb => sample_rgb(
        freq, scan_start, per_comp, mode.width, row, mode.width, &mut rgba,
      ),
      ColorKind::Yc => sample_yc(
        freq, scan_start, per_comp, mode.width, row, mode.width, &mut rgba,
      ),
    }
  }

  Ok(SstvImage {
    width: mode.width as u32,
    height: height as u32,
    mode: mode.name.to_owned(),
    rgba,
    source_sample_rate: source_rate,
    duration_seconds: freq.len() as f64 / WORK_RATE as f64,
  })
}

/// 在 `from` 起找第一个同步脉冲（1200 Hz，时长约 `sync_samples`），返回其起始位置。
fn find_sync(freq: &[f32], from: usize, sync_samples: usize) -> Option<usize> {
  let need = (sync_samples / 2).max(1);
  let mut run = 0usize;
  for (i, &f) in freq.iter().enumerate().skip(from) {
    if f < 1300.0 {
      run += 1;
      if run >= need {
        return Some(i + 1 - run);
      }
    } else {
      run = 0;
    }
  }
  None
}

/// 在预期位置 `expect` 附近重锁同步脉冲起点。
fn relock(freq: &[f32], expect: usize, sync_samples: usize) -> usize {
  let search = sync_samples.max(1);
  let lo = expect.saturating_sub(search);
  let hi = (expect + search).min(freq.len());
  let need = (sync_samples / 2).max(1);
  let mut best = expect;
  let mut best_run = 0usize;
  let mut run = 0usize;
  for (i, &f) in freq.iter().enumerate().take(hi).skip(lo) {
    if f < 1300.0 {
      run += 1;
      if run > best_run {
        best_run = run;
        best = i + 1 - run;
      }
    } else {
      run = 0;
    }
  }
  if best_run >= need { best } else { expect }
}

/// RGB 采样：每像素依次读 R、G、B 三个分量。
fn sample_rgb(
  freq: &[f32],
  scan_start: usize,
  per_comp: usize,
  width: usize,
  row: usize,
  stride: usize,
  rgba: &mut [u8],
) {
  for col in 0..width {
    let mut rgb = [0u8; 3];
    for (c, out) in rgb.iter_mut().enumerate() {
      let s = scan_start + (col * 3 + c) * per_comp;
      *out = gray(mean(&freq[s..(s + per_comp).min(freq.len())]));
    }
    let o = (row * stride + col) * 4;
    rgba[o] = rgb[0];
    rgba[o + 1] = rgb[1];
    rgba[o + 2] = rgb[2];
    rgba[o + 3] = 255;
  }
}

/// YC 采样（Robot）：Y 全分辨率 + 两路色度各半分辨率，色度水平减半。
fn sample_yc(
  freq: &[f32],
  scan_start: usize,
  per_comp: usize,
  width: usize,
  row: usize,
  stride: usize,
  rgba: &mut [u8],
) {
  let y_start = scan_start;
  let v_start = y_start + width * per_comp;
  let u_start = v_start + (width / 2) * per_comp;

  let mut y = vec![0u8; width];
  let mut v = vec![0.0f32; width / 2];
  let mut u = vec![0.0f32; width / 2];

  for (col, yv) in y.iter_mut().enumerate() {
    let s = y_start + col * per_comp;
    *yv = gray(mean(&freq[s..(s + per_comp).min(freq.len())]));
  }
  for k in 0..(width / 2) {
    let s = v_start + k * per_comp;
    v[k] = chroma(mean(&freq[s..(s + per_comp).min(freq.len())]));
    let s = u_start + k * per_comp;
    u[k] = chroma(mean(&freq[s..(s + per_comp).min(freq.len())]));
  }

  for col in 0..width {
    let yy = y[col] as f32 / 255.0;
    let vv = v[col / 2];
    let uu = u[col / 2];
    // BT.601 逆变换。
    let r = (yy + 1.140 * vv).clamp(0.0, 1.0);
    let g = (yy - 0.395 * uu - 0.581 * vv).clamp(0.0, 1.0);
    let b = (yy + 2.032 * uu).clamp(0.0, 1.0);
    let o = (row * stride + col) * 4;
    rgba[o] = (r * 255.0).round() as u8;
    rgba[o + 1] = (g * 255.0).round() as u8;
    rgba[o + 2] = (b * 255.0).round() as u8;
    rgba[o + 3] = 255;
  }
}

/// 频率 → 灰度（1500 Hz 黑，2300 Hz 白）。
fn gray(v: f32) -> u8 {
  let t = ((v - BLACK_HZ) / (WHITE_HZ - BLACK_HZ)).clamp(0.0, 1.0);
  (t * 255.0).round() as u8
}

/// 频率 → 色度（1500 Hz 为零色度，范围约 ±1）。
fn chroma(v: f32) -> f32 {
  ((v - SYNC_HZ - 300.0) / 800.0).clamp(-1.0, 1.0)
}

fn mean(xs: &[f32]) -> f32 {
  if xs.is_empty() {
    return 0.0;
  }
  xs.iter().sum::<f32>() / xs.len() as f32
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gray_maps_black_white() {
    assert_eq!(gray(1500.0), 0);
    assert_eq!(gray(2300.0), 255);
    assert_eq!(gray(1900.0), 128);
  }

  #[test]
  fn chroma_zero_at_1500() {
    assert!((chroma(1500.0)).abs() < 1e-3);
  }
}
