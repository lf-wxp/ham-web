//! 线性插值重采样到工作采样率，降采样前做抗混叠低通。

use super::WORK_RATE;

/// 汉明窗 sinc 低通系数（直流增益归一化为 1）。
fn lowpass(cutoff: f32, taps: usize) -> Vec<f32> {
  let taps = (taps.max(3) | 1).min(255);
  let m = (taps - 1) / 2;
  let mut coeffs = Vec::with_capacity(taps);
  let mut sum = 0.0f32;
  for i in 0..taps {
    let n = i as isize - m as isize;
    let h = if n == 0 {
      2.0 * cutoff
    } else {
      (std::f32::consts::TAU * cutoff * n as f32).sin() / (std::f32::consts::PI * n as f32)
    };
    let w = 0.54 - 0.46 * (std::f32::consts::TAU * i as f32 / (taps - 1) as f32).cos();
    let c = h * w;
    coeffs.push(c);
    sum += c;
  }
  for c in &mut coeffs {
    *c /= sum;
  }
  coeffs
}

/// FIR 直接卷积（边界补零）。
fn fir(x: &[f32], coeffs: &[f32]) -> Vec<f32> {
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

/// 将任意采样率线性重采样到 [`WORK_RATE`]。
pub fn to_work_rate(samples: &[f32], source_rate: u32) -> Vec<f32> {
  if samples.is_empty() || source_rate == 0 {
    return Vec::new();
  }
  if source_rate == WORK_RATE {
    return samples.to_vec();
  }

  let filtered: Vec<f32>;
  let src = if source_rate > WORK_RATE {
    // 抗混叠：截止取目标奈奎斯特的 0.9 倍。
    let cutoff = (WORK_RATE as f32 * 0.45) / source_rate as f32;
    let coeffs = lowpass(cutoff, 33);
    filtered = fir(samples, &coeffs);
    filtered.as_slice()
  } else {
    samples
  };

  let ratio = WORK_RATE as f64 / source_rate as f64;
  let out_len = ((src.len() as f64) * ratio).floor() as usize;
  let mut out = Vec::with_capacity(out_len);
  let last = src.len() - 1;
  for i in 0..out_len {
    let pos = i as f64 / ratio;
    let p0 = pos.floor() as usize;
    let p1 = (p0 + 1).min(last);
    let frac = (pos - p0 as f64) as f32;
    out.push(src[p0] + (src[p1] - src[p0]) * frac);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn identity_when_same_rate() {
    let input = vec![0.0f32, 0.5, 1.0, 0.5];
    let out = to_work_rate(&input, WORK_RATE);
    assert_eq!(out, input);
  }

  #[test]
  fn downsamples_length() {
    let input = vec![0.0f32; 16_000];
    let out = to_work_rate(&input, 16_000);
    assert!((out.len() as i64 - 8_000).abs() <= 2);
  }
}
