//! 晶体振荡器：负载电容、频率牵引（pulling）与振荡回路参数。

use std::f64::consts::TAU;

/// 晶振频率牵引量（ppm）：由负载电容 CL 决定相对串联谐振频率的偏移。
///
/// Δf/f = C₁ / (2·(C₀ + CL))，其中 `c1_f` 为动态电容、`c0_f` 为并联（静）电容、
/// `cl_f` 为负载电容（法拉）。返回 ppm。
#[must_use]
pub fn crystal_pull_ppm(c1_f: f64, c0_f: f64, cl_f: f64) -> f64 {
  if c1_f <= 0.0 || c0_f <= 0.0 || cl_f < 0.0 {
    return f64::NAN;
  }
  c1_f / (2.0 * (c0_f + cl_f)) * 1e6
}

/// 两电容串联（Pierce/Colpitts 负载电容，另需加 PCB 杂散电容）。
///
/// CL = C₁·C₂/(C₁+C₂)。
#[must_use]
pub fn series_capacitance(c1_f: f64, c2_f: f64) -> f64 {
  if c1_f <= 0.0 || c2_f <= 0.0 {
    return f64::NAN;
  }
  c1_f * c2_f / (c1_f + c2_f)
}

/// 串联谐振频率（Hz）：fs = 1/(2π√(L·C))。
///
/// `l_h` 电感（亨利）、`c_f` 电容（法拉）。
#[must_use]
pub fn series_resonance_hz(l_h: f64, c_f: f64) -> f64 {
  if l_h <= 0.0 || c_f <= 0.0 {
    return f64::NAN;
  }
  1.0 / (TAU * (l_h * c_f).sqrt())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pull_is_typically_tens_of_ppm() {
    // 典型 AT 切晶振：C1 ≈ 20 fF、C0 ≈ 5 pF，CL 从 20 pF 拉到 30 pF。
    let p1 = crystal_pull_ppm(20e-15, 5e-12, 20e-12);
    let p2 = crystal_pull_ppm(20e-15, 5e-12, 30e-12);
    // 约 400 ppm 与 285 ppm 量级，且 CL 越大牵引越小。
    assert!(p1 > 300.0 && p1 < 500.0, "p1={p1}");
    assert!(p2 < p1);
  }

  #[test]
  fn series_capacitance_smaller_than_either() {
    let c = series_capacitance(30e-12, 30e-12);
    assert!((c - 15e-12).abs() < 1e-15);
  }

  #[test]
  fn resonance_spot_check() {
    // 1 μH 与 1 nF 谐振于约 5.03 MHz。
    let f = series_resonance_hz(1e-6, 1e-9);
    assert!((f - 5.03e6).abs() < 0.02e6, "f={f}");
  }

  #[test]
  fn invalid_inputs_return_nan() {
    assert!(crystal_pull_ppm(0.0, 5e-12, 20e-12).is_nan());
    assert!(series_capacitance(0.0, 30e-12).is_nan());
    assert!(series_resonance_hz(0.0, 1e-9).is_nan());
  }
}
