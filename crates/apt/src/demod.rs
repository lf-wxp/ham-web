//! 2400 Hz 副载波的正交 AM 解调：混频 + 低通取包络。

use super::{WORD_RATE, WORK_RATE, filter};

/// 正交 AM 解调，返回视频包络（采样率仍为 `rate`）。
///
/// 通过 I/Q 混频并对平方和开方得到与载波相位无关的幅度包络。
pub fn am_demodulate(samples: &[f32], rate: u32, carrier: f32) -> Vec<f32> {
  // 低通截止取约 2000 Hz，足以容纳视频与 1040 Hz 的 sync A，同时抑制 2×载波分量。
  let coeffs = filter::lowpass(2000.0 / rate as f32, 65);

  let mut i = vec![0.0f32; samples.len()];
  let mut q = vec![0.0f32; samples.len()];
  let step = std::f32::consts::TAU * carrier / rate as f32;
  let mut phase = 0.0f32;
  for (n, &x) in samples.iter().enumerate() {
    let (s, c) = phase.sin_cos();
    i[n] = x * c;
    q[n] = x * s;
    // 用取模替代「减一次 TAU」，即使 step 较大（低采样率 + 高载波）也不会相位漂移。
    phase = (phase + step) % std::f32::consts::TAU;
  }

  let i_f = filter::apply(&i, &coeffs);
  let q_f = filter::apply(&q, &coeffs);
  // 正交解调对幅度有 1/2 增益，乘 2 还原真实包络幅度。
  i_f
    .iter()
    .zip(&q_f)
    .map(|(&a, &b)| 2.0 * (a * a + b * b).sqrt())
    .collect()
}

/// 将工作采样率的包络下采样到字速率（每 `WORK_RATE / WORD_RATE` 个取 1 个）。
pub fn to_words(envelope: &[f32]) -> Vec<f32> {
  let step = (WORK_RATE / WORD_RATE) as usize;
  envelope.iter().step_by(step).copied().collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recovers_am_envelope() {
    let rate = 20_800u32;
    let carrier = 2400.0f32;
    let n = 20_800usize; // 1 秒
    let mut env = vec![0.0f32; n];
    // 包络：0.5 秒内从 0.2 线性升到 0.8
    for (i, e) in env.iter_mut().enumerate() {
      *e = 0.2 + 0.6 * (i as f32 / n as f32);
    }
    let mut signal = vec![0.0f32; n];
    for i in 0..n {
      signal[i] = env[i] * (std::f32::consts::TAU * carrier * i as f32 / rate as f32).cos();
    }
    let demod = am_demodulate(&signal, rate, carrier);
    // 中段包络应接近 0.5（忽略滤波器建立的前几个样本）
    let mid = demod[n / 2];
    assert!((mid - 0.5).abs() < 0.05, "包络应约 0.5，得到 {mid}");
  }
}
