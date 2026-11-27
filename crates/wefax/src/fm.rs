//! 短波传真的调频副载波：1500 Hz = 黑、2300 Hz = 白（中心 1900 Hz、偏移 ±400 Hz）。
//!
//! 真实 HF 气象传真走 SSB（USB），音频就是这条调频副载波，而不是「幅度 = 亮度」的基带视频。
//! 本模块做两件事：把灰度图调成这种副载波（供合成对拍），以及把它解调回 `0..=1` 的亮度
//! （黑 = 1、白 = 0，与 [`crate::BLACK`] / [`crate::WHITE`] 同一口径），再交给行同步。
//!
//! # 为什么不用「混频 + 低通 + 相位差」
//!
//! 每个像素只有 10 个样本（1.25 ms），低通会把相邻像素的频率抹进来，黑白交界处出现
//! 灰边——对一张传真图，那是最常见的内容。这里改用线性预测：纯正弦满足
//! `x[n+1] + x[n-1] = 2·cos(ω)·x[n]`，对像素内部的 8 组三连样本做最小二乘就得到 `cos ω`，
//! 只用本像素自己的样本、与幅度无关（自带 AGC：录音留 headroom 也一样）。
//! 代价是抗噪声比平滑法差——见 `tests::noise_costs_accuracy_but_not_the_image`。
//!
//! # 没有建模的
//!
//! - 真实传真开头的 300 / 450 Hz 起始音与相位线（只靠每行同步脉冲找行首）；
//! - LSB 接收（黑白会倒过来，需先把边带翻回 USB）；
//! - 发端时钟与本机时钟的偏差（行长按标称值切，长录音会斜）；
//! - 卫星 WEFAX 的 2400 Hz 调幅副载波。

use crate::{
  Error, LINE_SAMPLES, PIXEL_SAMPLES, PIXELS_PER_LINE, SAMPLE_RATE, SYNC_SAMPLES, WefaxImage,
};

/// 黑电平的副载波频率（Hz）。
pub const BLACK_HZ: f64 = 1500.0;
/// 白电平的副载波频率（Hz）。
pub const WHITE_HZ: f64 = 2300.0;
/// 黑到白的频偏（Hz）。
pub const DEVIATION_HZ: f64 = WHITE_HZ - BLACK_HZ;
/// 合成时的载波幅度（留 10% headroom，转 16 位 WAV 不削波）。
const CARRIER_AMPLITUDE: f32 = 0.9;
/// 同步脉冲实测黑电平与标称 1500 Hz 相差在这个范围内才采信：这是调谐偏差，不是别的信号。
const TUNING_TOLERANCE_HZ: f64 = 150.0;
/// 判「像调频副载波」：窗口估出的频率落在 `[BLACK_HZ − 这个值, WHITE_HZ + 这个值]` 内。
const DETECT_MARGIN_HZ: f64 = 200.0;
/// 窗口能量低于它（均方）就不参与估计：静音的 `cos ω` 是 0/0。
const MIN_MEAN_SQUARE: f64 = 1e-8;
/// 估计调谐偏差与对齐时最多看多少行。
const CALIBRATION_LINES: usize = 8;

/// 灰度图 → 调频副载波采样（8000 Hz，相位连续；每行：1500 Hz 同步脉冲 + 逐像素的频率）。
pub fn modulate(image: &WefaxImage) -> Result<Vec<f32>, Error> {
  modulate_shifted(image, 0.0)
}

/// 同 [`modulate`]，整条副载波再平移 `shift_hz`（模拟接收机调谐不准）。
pub(crate) fn modulate_shifted(image: &WefaxImage, shift_hz: f64) -> Result<Vec<f32>, Error> {
  crate::check_image(image)?;
  let rate = f64::from(SAMPLE_RATE);
  let mut out = Vec::with_capacity(image.height * LINE_SAMPLES);
  let mut phase = 0.0_f64;
  let mut emit = |hz: f64, count: usize, out: &mut Vec<f32>| {
    let step = std::f64::consts::TAU * hz / rate;
    for _ in 0..count {
      out.push(CARRIER_AMPLITUDE * phase.sin() as f32);
      phase = (phase + step) % std::f64::consts::TAU;
    }
  };
  for row in 0..image.height {
    emit(BLACK_HZ + shift_hz, SYNC_SAMPLES, &mut out);
    for col in 0..image.width {
      let level = f64::from(image.gray[row * image.width + col]) / 255.0;
      emit(
        WHITE_HZ + shift_hz - DEVIATION_HZ * level,
        PIXEL_SAMPLES,
        &mut out,
      );
    }
  }
  Ok(out)
}

/// 一个窗口里正弦的 `cos ω`：对内部每个三连样本做最小二乘。窗口无能量时 `None`。
fn tone_cos(window: &[f32]) -> Option<f64> {
  let (sxx, syx, _) = moments(window)?;
  Some((syx / (2.0 * sxx)).clamp(-1.0, 1.0))
}

/// `(Σx², Σx·(前后之和), Σ(前后之和)²)`，对窗口内部的每个样本求和。
fn moments(window: &[f32]) -> Option<(f64, f64, f64)> {
  if window.len() < 3 {
    return None;
  }
  let (mut sxx, mut syx, mut syy) = (0.0_f64, 0.0_f64, 0.0_f64);
  for n in 1..window.len() - 1 {
    let x = f64::from(window[n]);
    let y = f64::from(window[n - 1]) + f64::from(window[n + 1]);
    sxx += x * x;
    syx += x * y;
    syy += y * y;
  }
  (sxx / (window.len() - 2) as f64 > MIN_MEAN_SQUARE).then_some((sxx, syx, syy))
}

/// 窗口里的副载波频率（Hz）；静音时 `None`。
pub(crate) fn window_hz(window: &[f32]) -> Option<f64> {
  tone_cos(window).map(|c| c.acos() * f64::from(SAMPLE_RATE) / std::f64::consts::TAU)
}

/// 频率 → 亮度（黑 = 1、白 = 0），`black_hz` 是这路信号实测的黑电平。
pub(crate) fn level_at(hz: f64, black_hz: f64) -> f32 {
  (((black_hz + DEVIATION_HZ - hz) / DEVIATION_HZ) as f32).clamp(0.0, 1.0)
}

/// 逐样本的亮度轨迹：`track[i]` 是以 `i` 起头的一个像素窗口的亮度（尾部不足一窗补 0）。
///
/// 只用来找行同步，所以按标称 1500 Hz 算；像素本身在对齐、校准之后再按实测黑电平重算。
pub(crate) fn track(samples: &[f32]) -> Vec<f32> {
  let mut out = vec![0.0_f32; samples.len()];
  if samples.len() < PIXEL_SAMPLES {
    return out;
  }
  for (i, window) in samples.windows(PIXEL_SAMPLES).enumerate() {
    // 频带外（直流、噪声）的窗口不算黑：否则调幅音频被强行按调频解时，0 Hz 会被折成
    // 「比黑还黑」，整段都像同步脉冲。
    if let Some(hz) = window_hz(window).filter(|&hz| in_band(hz)) {
      out[i] = level_at(hz, BLACK_HZ);
    }
  }
  out
}

/// 频率是否落在副载波的工作带内（含两侧的调谐余量）。
fn in_band(hz: f64) -> bool {
  (BLACK_HZ - DETECT_MARGIN_HZ..=WHITE_HZ + DETECT_MARGIN_HZ).contains(&hz)
}

/// 这段音频像不像调频副载波：多数有能量的窗口，其频率落在 1500 / 2300 Hz 附近。
///
/// 「幅度 = 亮度」的基带视频在每个像素里近似直流（`cos ω ≈ 1`，即 0 Hz），占不到多数。
/// 白噪声的预测系数在 0 附近、折成约 2 kHz，恰在带内，所以会被判成调频——没关系，
/// 它进了调频路径照样找不到同步脉冲，报的是同一个 `NoSync`。
pub(crate) fn looks_fm(samples: &[f32]) -> bool {
  let (mut seen, mut in_band_windows) = (0usize, 0usize);
  let (windows, _) = samples.as_chunks::<PIXEL_SAMPLES>();
  for window in windows {
    let Some(hz) = window_hz(window) else {
      continue;
    };
    seen += 1;
    if in_band(hz) {
      in_band_windows += 1;
    }
  }
  in_band_windows * 2 > seen
}

/// 以 `start` 为行首、把前几行切成 10 样本一格时，各格「不像单一正弦」的程度之和。
///
/// 格子与像素对齐时每格都是一段纯音、残差为 0；错开一两个样本，格子就跨进相邻像素，
/// 残差立刻变大。线性预测的像素值只在对齐时才准，所以行首要用它在同步脉冲附近微调。
pub(crate) fn misalignment(samples: &[f32], start: usize) -> Option<f64> {
  let lines = (samples.len().checked_sub(start)? / LINE_SAMPLES).min(CALIBRATION_LINES);
  if lines == 0 {
    return None;
  }
  let mut total = 0.0_f64;
  for line in 0..lines {
    let base = start + line * LINE_SAMPLES;
    for cell in 0..(SYNC_SAMPLES / PIXEL_SAMPLES + PIXELS_PER_LINE) {
      let off = base + cell * PIXEL_SAMPLES;
      let window = &samples[off..off + PIXEL_SAMPLES];
      let Some((sxx, syx, syy)) = moments(window) else {
        continue;
      };
      let c = (syx / (2.0 * sxx)).clamp(-1.0, 1.0);
      let residual = syy - 4.0 * c * syx + 4.0 * c * c * sxx;
      total += residual.max(0.0) / syy.max(f64::MIN_POSITIVE);
    }
  }
  Some(total)
}

/// 在候选行首里挑与像素对齐得最好的那个；残差相差不到 5%（加一点绝对余量）的并列取最早的。
///
/// 并列是真实存在的：线性预测只用一个窗口的前 `W − 1` 个样本做增量，窗口最后一个样本
/// 属于哪个像素都不影响拟合，所以晚一个样本的窗口同样「纯」，像素值也一样准。取最早的
/// 才不会把整幅图的最后一行平白舍掉。
pub(crate) fn best_alignment(samples: &[f32], candidates: std::ops::Range<usize>) -> Option<usize> {
  let scored: Vec<(usize, f64)> = candidates
    .filter_map(|c| misalignment(samples, c).map(|m| (c, m)))
    .collect();
  let best = scored.iter().map(|&(_, m)| m).min_by(f64::total_cmp)?;
  scored
    .iter()
    .find(|&&(_, m)| m <= best * 1.05 + 1e-6)
    .map(|&(c, _)| c)
}

/// 同步脉冲实测的黑电平（Hz）：前几行所有同步格的频率取中位数。
///
/// 传真每行同步脉冲都是黑，所以它直接告诉我们这路信号的黑落在哪（接收机调谐差几十
/// Hz 很常见）；白按「黑 + 800 Hz」推出。偏差超过 [`TUNING_TOLERANCE_HZ`] 就不信，
/// 按标称值走。
pub(crate) fn measured_black_hz(samples: &[f32], start: usize) -> f64 {
  let lines = (samples.len().saturating_sub(start) / LINE_SAMPLES).min(CALIBRATION_LINES);
  let mut hz = Vec::with_capacity(lines * SYNC_SAMPLES / PIXEL_SAMPLES);
  for line in 0..lines {
    let base = start + line * LINE_SAMPLES;
    for cell in 0..SYNC_SAMPLES / PIXEL_SAMPLES {
      let off = base + cell * PIXEL_SAMPLES;
      if let Some(f) = window_hz(&samples[off..off + PIXEL_SAMPLES]) {
        hz.push(f);
      }
    }
  }
  if hz.is_empty() {
    return BLACK_HZ;
  }
  hz.sort_by(f64::total_cmp);
  let median = hz[hz.len() / 2];
  if (median - BLACK_HZ).abs() <= TUNING_TOLERANCE_HZ {
    median
  } else {
    BLACK_HZ
  }
}
