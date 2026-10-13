//! 根升余弦（RRC）匹配滤波与 288 kHz OQPSK 采样。
//!
//! 滤波器参数转写自 lrpt-encoder（MIT）`modulation.rs`：滚降 0.5、跨度 33 符号、
//! 每符号 4 样本（288 kHz @ 72k 符号/s）、能量归一。

/// 滚降系数。
pub const ROLLOFF: f64 = 0.5;
/// 滤波器跨度（符号）。
pub const SPAN: usize = 33;
/// 每符号样本数（288 kHz / 72k）。
pub const SPS: usize = 4;

/// RRC 滤波器系数（`SPAN × SPS + 1` 抽头，能量归一）。
#[must_use]
pub fn rrc_taps() -> Vec<f32> {
  let num_taps = SPAN * SPS + 1;
  let half = (num_taps / 2) as f32;
  let alpha = ROLLOFF as f32;
  let mut taps = Vec::with_capacity(num_taps);
  for i in 0..num_taps {
    let t = (i as f32 - half) / SPS as f32;
    let val = if t.abs() < 1e-10 {
      1.0 - alpha + 4.0 * alpha / std::f32::consts::PI
    } else if (t.abs() - 1.0 / (4.0 * alpha)).abs() < 1e-10 {
      alpha / 2.0f32.sqrt()
        * ((1.0 + 2.0 / std::f32::consts::PI) * (std::f32::consts::PI / (4.0 * alpha)).sin()
          + (1.0 - 2.0 / std::f32::consts::PI) * (std::f32::consts::PI / (4.0 * alpha)).cos())
    } else {
      let pi_t = std::f32::consts::PI * t;
      let four_alpha_t = 4.0 * alpha * t;
      (((1.0 - alpha) * pi_t).sin() + four_alpha_t * ((1.0 + alpha) * pi_t).cos())
        / (pi_t * (1.0 - four_alpha_t * four_alpha_t))
    };
    taps.push(val);
  }
  let energy: f32 = taps.iter().map(|x| x * x).sum();
  let norm = 1.0 / energy.sqrt();
  taps.iter_mut().for_each(|t| *t *= norm);
  taps
}

/// 匹配滤波：I 与 Q 通道**分别**做 FIR（抽头间距按每通道每复数样本 1 个浮点
/// 定义，交织流直接卷积会把抽头间距错成一半）。
#[must_use]
pub fn matched_filter(iq: &[f32]) -> (Vec<f32>, Vec<f32>) {
  let taps = rrc_taps();
  let n = taps.len();
  let m = iq.len() / 2;
  if m < n {
    return (Vec::new(), Vec::new());
  }
  let conv = |ch: &[f32]| -> Vec<f32> {
    let mut out = Vec::with_capacity(ch.len() - n + 1);
    for i in 0..=ch.len() - n {
      let mut acc = 0.0f32;
      for (k, &t) in taps.iter().enumerate() {
        acc += ch[i + k] * t;
      }
      out.push(acc);
    }
    out
  };
  let i_ch: Vec<f32> = iq.iter().step_by(2).copied().collect();
  let q_ch: Vec<f32> = iq.iter().skip(1).step_by(2).copied().collect();
  (conv(&i_ch), conv(&q_ch))
}

/// 288 kHz I/Q 样本 → 硬判决位流（每符号采样一次）。
///
/// I 与 Q 都在符号网格上采样：Q 的半符号滞后**在波形里**，不是采样偏移
/// （实测对拍：Iφ=0、Qφ=0、延迟 33 符号 → 0 误差）。
/// 输出对应「输入第 `FILTER_DELAY_SYMBOLS + k` 个符号」，调用方要丢弃前
/// [`FILTER_DELAY_SYMBOLS`] 个符号的位。
///
/// 本函数只做单相位；`decode_stream288` 的相位搜索（0..SPS）走 [`matched_filter`] +
/// [`demodulate_288_phase`]，滤波只做一次。
#[must_use]
pub fn demodulate_288(iq: &[f32], phase: usize) -> Vec<u8> {
  let (fi, fq) = matched_filter(iq);
  demodulate_288_phase(&fi, &fq, phase)
}

/// 已滤波 I/Q → 硬判决位流（每符号采样一次，`phase` 为采样相位 0..SPS）。
#[must_use]
pub fn demodulate_288_phase(fi: &[f32], fq: &[f32], phase: usize) -> Vec<u8> {
  // `phase` 来自公共 API：越界值（如 `usize::MAX`）会让 `i + SPS / 2` 回绕后索引越界。
  if phase >= SPS {
    return Vec::new();
  }
  let mut out = Vec::new();
  let mut i = phase;
  // `min(fi.len())`：公共 API 不假设 fi/fq 等长，防越界。
  while i + SPS / 2 < fq.len().min(fi.len()) {
    // 极性约定对齐 lrpt-encoder：+1 → bit 0；Q 滞后半个符号（SPS/2 样本）。
    out.push(if fi[i] < 0.0 { 1 } else { 0 });
    out.push(if fq[i + SPS / 2] < 0.0 { 1 } else { 0 });
    i += SPS;
  }
  out
}

/// 两级 RRC 滤波器的群延迟（符号数）：`(2 × 抽头数 − 2) / (2 × SPS)` 取整
/// （数值上等于 SPAN，但按群延迟公式写，避免依赖巧合相等）。
pub const FILTER_DELAY_SYMBOLS: usize = (2 * (SPAN * SPS + 1) - 2) / (2 * SPS);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn taps_are_normalized_and_centered() {
    let taps = rrc_taps();
    assert_eq!(taps.len(), SPAN * SPS + 1);
    let energy: f32 = taps.iter().map(|x| x * x).sum();
    assert!((energy - 1.0).abs() < 1e-4, "能量归一：{energy}");
    // 中心对称。
    for k in 0..taps.len() / 2 {
      assert!((taps[k] - taps[taps.len() - 1 - k]).abs() < 1e-6);
    }
  }
}
