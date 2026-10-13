//! 解码器共用的 DSP 原语。
//!
//! RTTY / Olivia / DominoEX / NAVTEX 各自曾内联一份逐字相同的 Goertzel；统一到这里，
//! 修一处就够了。刻意只放「**与具体调制无关**」的东西：`wspr` 的 Goertzel 按窗口长度取整到
//! 频点（语义不同），不在此列。

/// Goertzel：`samples` 在 `freq`（Hz）处的功率（未归一化，量纲 = 幅度² × 窗口长度²）。
///
/// 与按整数频点取整的 DFT 单点不同，这里直接用 `freq / rate` 的**非整数**角频率，所以
/// 可以对任意频率（含 1 Hz 步进的音调搜索）求值。`rate == 0` 或非有限频率得到 `NaN` /
/// `inf`，调用方负责事先拒绝（各解码器入口已有守卫）。
#[must_use]
pub fn tone_power(samples: &[f32], rate: u32, freq: f64) -> f64 {
  let w = 2.0 * std::f64::consts::PI * freq / f64::from(rate);
  let coeff = 2.0 * w.cos();
  let mut s0 = 0.0f64;
  let mut s1 = 0.0f64;
  for &x in samples {
    let s = f64::from(x) + coeff * s1 - s0;
    s0 = s1;
    s1 = s;
  }
  s1 * s1 + s0 * s0 - coeff * s1 * s0
}

#[cfg(test)]
mod tests {
  use super::tone_power;

  fn sine(freq: f64, rate: u32, n: usize) -> Vec<f32> {
    (0..n)
      .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin() as f32)
      .collect()
  }

  #[test]
  fn peaks_at_the_tone_frequency() {
    let x = sine(1000.0, 8000, 800);
    let on = tone_power(&x, 8000, 1000.0);
    let off = tone_power(&x, 8000, 1500.0);
    assert!(on > off * 100.0, "on={on} off={off}");
  }

  #[test]
  fn silence_has_no_power() {
    assert_eq!(tone_power(&[0.0; 256], 8000, 1000.0), 0.0);
    assert_eq!(tone_power(&[], 8000, 1000.0), 0.0);
  }

  #[test]
  fn power_scales_with_amplitude_squared() {
    let a = sine(700.0, 8000, 800);
    let b: Vec<f32> = a.iter().map(|v| v * 2.0).collect();
    let ratio = tone_power(&b, 8000, 700.0) / tone_power(&a, 8000, 700.0);
    assert!((ratio - 4.0).abs() < 1e-6, "{ratio}");
  }
}
