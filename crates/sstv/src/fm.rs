//! FM 解调：Hilbert 变换 + 瞬时相位差分，得到瞬时频率（Hz）。

use std::f32::consts::{PI, TAU};

/// Hilbert 变换 FIR 系数（奇数长度，系数 -2/(πn)，n 为奇数，产生 +90° 相移）。
///
/// 取负号使得解析信号 `x + j·x_h` 对正频率信号表现为 `e^{jωn}`（瞬时频率为正）。
pub fn hilbert_coeffs(taps: usize) -> Vec<f32> {
  let taps = (taps | 1).max(3);
  let m = (taps as isize - 1) / 2;
  let mut h = vec![0.0f32; taps];
  for (n, c) in h.iter_mut().enumerate() {
    let k = n as isize - m;
    if k % 2 != 0 {
      *c = -2.0 / (PI * k as f32);
    }
  }
  h
}

/// FIR 直接卷积（边界补零），返回与输入等长的滤波结果。
pub fn fir(x: &[f32], coeffs: &[f32]) -> Vec<f32> {
  let n = x.len();
  let mut out = vec![0.0f32; n];
  let m = (coeffs.len() - 1) / 2;
  for (i, o) in out.iter_mut().enumerate() {
    let mut acc = 0.0f32;
    for (k, &c) in coeffs.iter().enumerate() {
      let idx = i as isize + k as isize - m as isize;
      if idx >= 0 && (idx as usize) < n {
        acc += x[idx as usize] * c;
      }
    }
    *o = acc;
  }
  out
}

/// FM 解调：返回瞬时频率（Hz），长度与输入相同。
///
/// 用 Hilbert 变换构造解析信号，再对瞬时相位做差分（含 unwrap）。
pub fn fm_demodulate(x: &[f32], rate: u32) -> Vec<f32> {
  let n = x.len();
  if n == 0 {
    return Vec::new();
  }
  let coeffs = hilbert_coeffs(127);
  let x_h = fir(x, &coeffs);

  let mut freq = vec![0.0f32; n];
  let scale = rate as f32 / TAU;
  let mut prev = x_h[0].atan2(x[0]);
  for i in 0..n {
    let phase = x_h[i].atan2(x[i]);
    let mut d = phase - prev;
    while d > PI {
      d -= TAU;
    }
    while d < -PI {
      d += TAU;
    }
    freq[i] = d * scale;
    prev = phase;
  }
  freq
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recovers_tone_frequency() {
    let rate = 8000u32;
    let f = 1900.0f32;
    let n = rate as usize; // 1 秒
    let signal: Vec<f32> = (0..n)
      .map(|i| (TAU * f * i as f32 / rate as f32).sin())
      .collect();
    let freq = fm_demodulate(&signal, rate);
    // 跳过边界建立段，取中段平均。
    let mid = &freq[n / 4..3 * n / 4];
    let avg: f32 = mid.iter().sum::<f32>() / mid.len() as f32;
    assert!((avg - f).abs() < 30.0, "解调频率应接近 {f} Hz，得到 {avg}");
  }

  #[test]
  fn distinguishes_1200_and_1300() {
    let rate = 8000u32;
    let n = rate as usize / 2;
    let f1 = 1200.0f32;
    let f2 = 1300.0f32;
    let mut signal = Vec::with_capacity(n * 2);
    for i in 0..n {
      signal.push((TAU * f1 * i as f32 / rate as f32).sin());
    }
    for i in 0..n {
      signal.push((TAU * f2 * i as f32 / rate as f32).sin());
    }
    let freq = fm_demodulate(&signal, rate);
    let a: f32 = freq[n / 4..n / 2].iter().sum::<f32>() / (n / 4) as f32;
    let b: f32 = freq[5 * n / 4..3 * n / 2].iter().sum::<f32>() / (n / 4) as f32;
    assert!((a - f1).abs() < 30.0, "第一段应约 {f1} Hz，得到 {a}");
    assert!((b - f2).abs() < 30.0, "第二段应约 {f2} Hz，得到 {b}");
  }
}
