//! 简单 FIR 低通滤波（汉明窗 sinc），用于抗混叠与 AM 解调中的低通。

/// 设计归一化截止频率 `cutoff`（0..0.5，相对奈奎斯特）的汉明窗 sinc 低通，
/// 返回奇数个系数，直流增益归一化为 1。
pub fn lowpass(cutoff: f32, taps: usize) -> Vec<f32> {
  let taps = (taps.max(3) | 1).min(255); // 强制奇数并设上限
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

/// 直接卷积（边界补零），返回与输入等长的滤波结果。
pub fn apply(samples: &[f32], coeffs: &[f32]) -> Vec<f32> {
  let n = samples.len();
  let mut out = vec![0.0f32; n];
  let m = (coeffs.len() - 1) / 2;
  for (i, o) in out.iter_mut().enumerate() {
    let mut acc = 0.0f32;
    for (k, &c) in coeffs.iter().enumerate() {
      let idx = i as isize + k as isize - m as isize;
      if idx >= 0 && (idx as usize) < n {
        acc += samples[idx as usize] * c;
      }
    }
    *o = acc;
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dc_gain_is_unity() {
    let coeffs = lowpass(0.1, 33);
    let input = vec![1.0f32; 256];
    let out = apply(&input, &coeffs);
    let mid = out[128];
    assert!((mid - 1.0).abs() < 1e-3, "DC 增益应为 1，得到 {mid}");
  }

  #[test]
  fn attenuates_high_frequency() {
    let coeffs = lowpass(0.1, 65);
    // 奈奎斯特频率（2 样本/周期）正弦：0,1,0,-1,...
    let input: Vec<f32> = (0..256)
      .map(|i| ((i as f32) * std::f32::consts::FRAC_PI_2).sin())
      .collect();
    let out = apply(&input, &coeffs);
    // 只测稳态（跳过边界建立段）；直流增益为 1，奈奎斯特应被明显衰减。
    let peak = out[100..].iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    assert!(peak < 0.25, "高频应被显著衰减（相对直流 1.0），峰值 {peak}");
  }
}
