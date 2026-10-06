//! 频谱与瀑布图的数据处理：线性幅度 → dB、伪彩色映射与峰值检测。
//!
//! 供浏览器内 SDR 瀑布图使用。颜色映射是「黑 → 蓝 → 青 → 绿 → 黄 → 橙 → 白」
//! 的分段线性伪彩色，与常见 SDR 软件的瀑布图配色一致。

use std::f32::consts::TAU;

use crate::fft::fft_in_place;

/// 瀑布图显示的最低 dB 值（对应黑色）。
pub const FLOOR_DB: f32 = -120.0;

/// 线性幅度（0–∞）→ 分贝（dBFS，20·log10）。
///
/// `v <= 0` 时返回 [`FLOOR_DB`]（视为底噪下限，避免负无穷）。
///
/// **入参必须是线性幅度**。若喂进来的已经是 dB 值（例如 Web Audio 的
/// `getFloatFrequencyData` 输出，恒为负数），本函数会对每个 bin 返回 [`FLOOR_DB`]，
/// 使整幅图静默变成一条平线而不报错。遇到「频谱图全黑但明显有信号」先查这里。
#[must_use]
pub fn db_from_linear(v: f32) -> f32 {
  if v <= 0.0 { FLOOR_DB } else { 20.0 * v.log10() }
}

/// 把 dB 值映射为瀑布图伪彩色 RGB。
///
/// `floor_db` 为最暗（黑色）对应的 dB，`ceil_db` 为最亮（白色）对应的 dB，
/// 两者之间按分段线性渐变。超出范围的输入会被裁剪到端点颜色。
#[must_use]
pub fn waterfall_color(db: f32, floor_db: f32, ceil_db: f32) -> (u8, u8, u8) {
  let span = ceil_db - floor_db;
  if span <= 0.0 {
    return (0, 0, 0);
  }
  let t = ((db - floor_db) / span).clamp(0.0, 1.0);
  // 分段线性控制点：(位置, R, G, B)
  const STOPS: &[(f32, u8, u8, u8)] = &[
    (0.00, 0, 0, 0),
    (0.15, 0, 0, 90),
    (0.35, 0, 90, 200),
    (0.50, 0, 200, 200),
    (0.65, 0, 220, 0),
    (0.80, 230, 220, 0),
    (0.90, 255, 120, 0),
    (1.00, 255, 255, 255),
  ];
  for w in STOPS.windows(2) {
    let (t0, r0, g0, b0) = w[0];
    let (t1, r1, g1, b1) = w[1];
    if t >= t0 && t <= t1 {
      let k = (t - t0) / (t1 - t0);
      #[allow(clippy::cast_possible_truncation)]
      let mix = |a: u8, b: u8| {
        let v = f32::from(a) + (f32::from(b) - f32::from(a)) * k;
        v.round().clamp(0.0, 255.0) as u8
      };
      return (mix(r0, r1), mix(g0, g1), mix(b0, b1));
    }
  }
  (255, 255, 255)
}

/// 频谱峰值（最大 bin 的 dB 值）。空输入返回 [`FLOOR_DB`]。
#[must_use]
pub fn peak_db(bins: &[f32]) -> f32 {
  bins.iter().fold(FLOOR_DB, |acc, &v| {
    let d = db_from_linear(v);
    if d > acc { d } else { acc }
  })
}

/// 合成一段测试信号：多个 (频率 Hz, 幅度) 正弦波叠加，可选加白噪声。
///
/// 用于生成频谱 / 瀑布图的合成测试样本（见 `cargo make spectrum-sample`）。
#[must_use]
pub fn synthesize(rate: u32, seconds: f32, tones: &[(f32, f32)], noise: f32) -> Vec<f32> {
  let n = (f64::from(rate) * f64::from(seconds)).round() as usize;
  let mut out = vec![0.0f32; n];
  for (i, s) in out.iter_mut().enumerate() {
    let t = i as f32 / rate as f32;
    let mut v = 0.0f32;
    for &(f, a) in tones {
      v += a * (TAU * f * t).sin();
    }
    *s = v;
  }
  if noise > 0.0 {
    let mut seed = 0x1234_5678u32;
    for s in &mut out {
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      let u = (seed >> 8) as f32 / 16_777_216.0;
      *s += noise * (u * 2.0 - 1.0);
    }
  }
  out
}

/// 计算周期（DFT-even）Hann 窗（长度 `n`）。
fn hann_window(n: usize) -> Vec<f32> {
  let denom = n as f32;
  (0..n)
    .map(|i| 0.5 - 0.5 * (TAU * i as f32 / denom).cos())
    .collect()
}

/// Hann 窗的等效噪声带宽（单位：bin）。
///
/// 相干增益补偿（[`spectrum_db`] 里的 `2/Σw`）只对**单音**成立。对噪声 / 宽带信号，
/// 每个 bin 的功率还要再乘 ENBW=1.5 才是真实值，即读数偏低 `10·lg1.5 ≈ 1.76 dB`。
/// 要把瀑布图上的「噪底」当成功率密度读数时，需补上 [`HANN_ENBW_CORRECTION_DB`]。
pub const HANN_ENBW_BINS: f32 = 1.5;

/// 加 Hann 窗后噪声类读数换算为真实功率所需的修正量（dB，加到读数上）。
pub const HANN_ENBW_CORRECTION_DB: f32 = 1.76;

/// 对一段实样本做 FFT，返回 0..N/2 的 dB 幅度谱（加 Hann 窗，N 为 ≤ 长度的最大 2 的幂）。
///
/// 归一化已补偿 Hann 窗的相干增益：幅度 1.0 的实正弦约读到 0 dB。
///
/// 该标定只对单音有效；读噪声电平时请另加 [`HANN_ENBW_CORRECTION_DB`]。
#[must_use]
pub fn spectrum_db(samples: &[f32]) -> Vec<f32> {
  let mut n = 1usize;
  while n * 2 <= samples.len() {
    n *= 2;
  }
  if n < 2 {
    return Vec::new();
  }
  let win = hann_window(n);
  let mut re = vec![0.0f32; n];
  let mut im = vec![0.0f32; n];
  for i in 0..n {
    re[i] = samples[i] * win[i];
  }
  fft_in_place(&mut re, &mut im);
  // 幅度为 A 的实正弦，加窗后峰值 bin 幅度约为 A·Σw/2，故取 2/Σw 还原为 A。
  let coherent = win.iter().sum::<f32>();
  let scale = if coherent > 0.0 { 2.0 / coherent } else { 0.0 };
  (0..n / 2)
    .map(|i| db_from_linear(re[i].hypot(im[i]) * scale))
    .collect()
}

/// 第 `bin` 个 bin 对应的频率（Hz）。
///
/// `bin_count` 传入的是**返回的 bin 数**（即 N/2），函数内部再乘 2 还原 FFT 长度。
/// 误传 FFT 长度 N 会让结果差一倍。
fn bin_frequency_hz(bin: usize, bin_count: usize, sample_rate: f32) -> f32 {
  bin as f32 * sample_rate / (bin_count * 2) as f32
}

/// 频谱中最大 bin 对应的频率（Hz）。空频谱返回 0。
#[must_use]
pub fn peak_frequency_hz(db: &[f32], sample_rate: f32) -> f32 {
  if db.is_empty() {
    return 0.0;
  }
  let peak = db
    .iter()
    .enumerate()
    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
    .map(|(i, _)| i)
    .unwrap();
  bin_frequency_hz(peak, db.len(), sample_rate)
}

/// 对整段样本做分帧频谱分析，生成瀑布图 RGBA 与平均频谱。
///
/// `frame_size` 为每帧 FFT 长度（2 的幂，决定瀑布图宽度 = `frame_size/2`），
/// `height` 为瀑布图行数。返回 `(瀑布图 RGBA 像素, 平均频谱 dB)`；瀑布图
/// 顶部对应最新时间，底部对应最早时间。
#[must_use]
pub fn analyze_waterfall(
  samples: &[f32],
  frame_size: usize,
  height: usize,
  floor_db: f32,
  ceil_db: f32,
) -> Option<(Vec<u8>, Vec<f32>)> {
  if samples.len() < frame_size || height == 0 || !frame_size.is_power_of_two() {
    return None;
  }
  let width = frame_size / 2;
  // 计算帧步进，使帧数尽量填满 `height` 行。
  let hop = if height > 1 {
    ((samples.len() - frame_size) as f64 / (height - 1) as f64)
      .ceil()
      .max(1.0) as usize
  } else {
    samples.len()
  };

  let mut rgba = vec![0u8; width * height * 4];
  let mut avg_db = vec![0f32; width];
  let mut count = 0usize;
  let mut frame_index = 0usize;
  let mut start = 0usize;
  while start + frame_size <= samples.len() && frame_index < height {
    let db = spectrum_db(&samples[start..start + frame_size]);
    let row = height - 1 - frame_index; // 顶部 = 最新帧
    for (i, &v) in db.iter().enumerate() {
      avg_db[i] += v;
      let (r, g, b) = waterfall_color(v, floor_db, ceil_db);
      let o = (row * width + i) * 4;
      rgba[o] = r;
      rgba[o + 1] = g;
      rgba[o + 2] = b;
      rgba[o + 3] = 255;
    }
    count += 1;
    frame_index += 1;
    start += hop;
  }
  if count > 0 {
    for v in &mut avg_db {
      *v /= count as f32;
    }
  }
  Some((rgba, avg_db))
}

/// 频谱 dB → 柱状 RGBA 图像（用于显示平均频谱，柱高随幅度增长）。
#[must_use]
pub fn spectrum_to_rgba(db: &[f32], height: usize, floor_db: f32, ceil_db: f32) -> Vec<u8> {
  let width = db.len();
  let mut rgba = vec![0u8; width * height * 4];
  let span = ceil_db - floor_db;
  if span <= 0.0 {
    return rgba;
  }
  for (i, &v) in db.iter().enumerate() {
    let t = ((v - floor_db) / span).clamp(0.0, 1.0);
    let h = (t * height as f32).round() as usize;
    for y in (height - h)..height {
      let o = (y * width + i) * 4;
      rgba[o] = 34;
      rgba[o + 1] = 211;
      rgba[o + 2] = 238;
      rgba[o + 3] = 255;
    }
  }
  rgba
}

/// 频谱中前 `k` 个峰值（按幅度降序，做主瓣抑制），返回 (频率 Hz, dB)。
///
/// 每取一个峰值后抑制其邻域（±2 bin，Hann 窗主瓣零点间半宽），避免同一音调的
/// 旁瓣被重复计为多个峰值。
#[must_use]
pub fn top_peaks(db: &[f32], sample_rate: f32, k: usize) -> Vec<(f32, f32)> {
  let n = db.len();
  if n == 0 {
    return Vec::new();
  }
  let mut used = vec![false; n];
  let mut peaks = Vec::with_capacity(k);
  for _ in 0..k {
    let mut best: Option<(usize, f32)> = None;
    for (i, &v) in db.iter().enumerate() {
      if used[i] {
        continue;
      }
      if best.is_none_or(|(_, bv)| v > bv) {
        best = Some((i, v));
      }
    }
    let Some((bi, bv)) = best else {
      break;
    };
    peaks.push((bin_frequency_hz(bi, n, sample_rate), bv));
    // Hann 窗主瓣零点到零点的半宽为 2 个 bin；抑制范围过大会把相距 ≤4 bin 的
    // 两个音合并成一个，故取 ±2。
    let lo = bi.saturating_sub(2);
    let hi = (bi + 2).min(n - 1);
    for u in &mut used[lo..=hi] {
      *u = true;
    }
  }
  peaks
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn db_from_linear_basics() {
    assert!((db_from_linear(1.0)).abs() < 1e-4);
    assert!((db_from_linear(0.1) + 20.0).abs() < 1e-4);
    assert_eq!(db_from_linear(0.0), FLOOR_DB);
    assert_eq!(db_from_linear(-1.0), FLOOR_DB);
  }

  #[test]
  fn color_maps_floor_and_ceil() {
    assert_eq!(waterfall_color(-100.0, -100.0, 0.0), (0, 0, 0));
    assert_eq!(waterfall_color(0.0, -100.0, 0.0), (255, 255, 255));
    // 低于 floor 裁剪为黑，高于 ceil 裁剪为白。
    assert_eq!(waterfall_color(-120.0, -100.0, 0.0), (0, 0, 0));
    assert_eq!(waterfall_color(10.0, -100.0, 0.0), (255, 255, 255));
  }

  #[test]
  fn color_brightness_increases() {
    let sum = |t: f32| {
      let (r, g, b) = waterfall_color(t, 0.0, 1.0);
      u32::from(r) + u32::from(g) + u32::from(b)
    };
    assert!(sum(0.75) > sum(0.25));
    assert!(sum(1.0) > sum(0.5));
    assert!(sum(0.5) > sum(0.0));
  }

  #[test]
  fn peak_db_finds_max() {
    let bins = [0.01f32, 0.5, 0.1, 0.9];
    let p = peak_db(&bins);
    assert!((p - db_from_linear(0.9)).abs() < 1e-4);
    assert_eq!(peak_db(&[]), FLOOR_DB);
  }

  #[test]
  fn spectrum_finds_synthesized_tones() {
    let rate = 8000u32;
    let samples = synthesize(rate, 1.0, &[(700.0, 0.8), (1500.0, 0.5)], 0.0);
    let db = spectrum_db(&samples);
    assert!(!db.is_empty());

    let bin_hz = rate as f32 / (db.len() * 2) as f32;
    let bin_of = |hz: f32| (hz / bin_hz).round() as usize;
    let b700 = bin_of(700.0);
    let b1500 = bin_of(1500.0);
    assert!(db[b700] > -20.0, "700 Hz bin {} = {} dB", b700, db[b700]);
    assert!(
      db[b1500] > -25.0,
      "1500 Hz bin {} = {} dB",
      b1500,
      db[b1500]
    );
    // 有音调处应显著高于无音调的中间 bin。
    let mid = bin_of(1050.0);
    assert!(db[b700] > db[mid] + 20.0);
  }

  #[test]
  fn peak_frequency_matches_tone() {
    let rate = 8000u32;
    let samples = synthesize(rate, 1.0, &[(1000.0, 0.9)], 0.0);
    let db = spectrum_db(&samples);
    let peak = peak_frequency_hz(&db, rate as f32);
    let bin_hz = rate as f32 / (db.len() * 2) as f32;
    assert!((peak - 1000.0).abs() <= bin_hz, "峰值 {peak} Hz");
  }

  #[test]
  fn top_peaks_returns_dominant_tones() {
    let rate = 8000u32;
    let samples = synthesize(rate, 1.0, &[(700.0, 0.9), (1500.0, 0.5)], 0.0);
    let db = spectrum_db(&samples);
    let peaks = top_peaks(&db, rate as f32, 2);
    assert_eq!(peaks.len(), 2);
    assert!(
      (peaks[0].0 - 700.0).abs() < 20.0,
      "第一峰 {} Hz",
      peaks[0].0
    );
    assert!(
      (peaks[1].0 - 1500.0).abs() < 20.0,
      "第二峰 {} Hz",
      peaks[1].0
    );
  }

  #[test]
  fn analyze_waterfall_produces_rows_and_peaks() {
    let rate = 8000u32;
    let samples = synthesize(rate, 2.0, &[(700.0, 0.8), (1500.0, 0.5)], 0.0);
    let (rgba, avg) = analyze_waterfall(&samples, 2048, 64, FLOOR_DB, 0.0).unwrap();
    assert_eq!(rgba.len(), 1024 * 64 * 4);
    assert_eq!(avg.len(), 1024);
    // 平均频谱应在 700 / 1500 Hz 出现峰值。
    let bin_hz = rate as f32 / 2048.0;
    let b700 = (700.0 / bin_hz).round() as usize;
    let b1500 = (1500.0 / bin_hz).round() as usize;
    assert!(avg[b700] > -20.0, "700 Hz bin {} = {} dB", b700, avg[b700]);
    assert!(
      avg[b1500] > -25.0,
      "1500 Hz bin {} = {} dB",
      b1500,
      avg[b1500]
    );
    // 有信号处应比无信号处亮（取第一帧所在的最底行）。
    let bottom = (64 - 1) * 1024 + b700;
    assert!(rgba[bottom * 4] > 0);
  }

  #[test]
  fn analyze_waterfall_rejects_short_or_bad_inputs() {
    let rate = 8000u32;
    let samples = synthesize(rate, 0.1, &[(700.0, 0.8)], 0.0);
    assert!(analyze_waterfall(&samples, 2048, 64, FLOOR_DB, 0.0).is_none());
    assert!(analyze_waterfall(&samples, 2000, 64, FLOOR_DB, 0.0).is_none()); // 非 2 的幂
  }

  #[test]
  fn spectrum_to_rgba_sizes() {
    let db = vec![-10.0f32; 100];
    let rgba = spectrum_to_rgba(&db, 50, FLOOR_DB, 0.0);
    assert_eq!(rgba.len(), 100 * 50 * 4);
    // ceil 处全亮（青），floor 处全暗。
    let full = spectrum_to_rgba(&[0.0f32], 10, FLOOR_DB, 0.0);
    assert_eq!(full[3], 255);
    let empty = spectrum_to_rgba(&[FLOOR_DB], 10, FLOOR_DB, 0.0);
    assert_eq!(&empty[..4], &[0, 0, 0, 0]);
  }
}
