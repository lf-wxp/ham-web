//! 射频暴露评估：按 FCC OET-65 计算最大允许暴露（MPE）功率密度与最小安全距离。
//!
//! 采用远场近似：功率密度 S = P·G / (4π d²)。近场区（距离小于数个波长）实际场强
//! 可能更高，故本计算仅作合规自检参考，不能替代专业电磁环境评估。

use std::f64::consts::PI;

/// 射频暴露评估结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RfExposure {
  /// 给定距离处的功率密度（mW/cm²，远场近似）。
  pub power_density: f64,
  /// 受控（职业）环境 MPE 限值（mW/cm²）。
  pub mpe_controlled: f64,
  /// 非受控（公众）环境 MPE 限值（mW/cm²）。
  pub mpe_uncontrolled: f64,
  /// 受控环境最小安全距离（米）。
  pub safe_distance_controlled_m: f64,
  /// 非受控环境最小安全距离（米）。
  pub safe_distance_uncontrolled_m: f64,
}

/// FCC OET-65 的 MPE 功率密度限值（mW/cm²）。
///
/// `controlled` 为 true 时返回职业/受控环境限值，否则为公众/非受控环境限值。
/// 频率低于 0.3 MHz 或高于 100 GHz 返回 0（超出模型适用范围）。
#[must_use]
pub fn mpe_limit_mw_cm2(freq_mhz: f64, controlled: bool) -> f64 {
  let f = freq_mhz;
  match f {
    f if f < 0.3 => 0.0,
    f if f < 3.0 => 100.0,
    f if f < 30.0 => {
      if controlled {
        900.0 / (f * f)
      } else {
        180.0 / (f * f)
      }
    }
    f if f < 300.0 => {
      if controlled {
        1.0
      } else {
        0.2
      }
    }
    f if f < 1500.0 => {
      if controlled {
        f / 300.0
      } else {
        f / 1500.0
      }
    }
    f if f <= 100_000.0 => {
      if controlled {
        5.0
      } else {
        1.0
      }
    }
    _ => 0.0,
  }
}

/// 天线增益（dBi）换算为线性倍数。
#[must_use]
pub fn linear_gain(gain_dbi: f64) -> f64 {
  10f64.powf(gain_dbi / 10.0)
}

/// 给定距离处的功率密度（mW/cm²，远场近似）。
///
/// S = P·G / (4π d²)。`power_w` 发射功率（瓦）、`gain_dbi` 天线增益、
/// `distance_m` 距离（米）。距离非正时返回正无穷。
#[must_use]
pub fn power_density_mw_cm2(power_w: f64, gain_dbi: f64, distance_m: f64) -> f64 {
  if distance_m <= 0.0 {
    return f64::INFINITY;
  }
  let p_mw = power_w * 1000.0;
  let g = linear_gain(gain_dbi);
  let r_cm2 = (distance_m * 100.0).powi(2);
  p_mw * g / (4.0 * PI * r_cm2)
}

/// 综合评估：返回功率密度、两类限值与最小安全距离。
///
/// 最小安全距离 = √(P·G / (4π · MPE))，从厘米换算回米。
#[must_use]
pub fn assess(power_w: f64, freq_mhz: f64, gain_dbi: f64, distance_m: f64) -> Option<RfExposure> {
  if power_w <= 0.0 || freq_mhz < 0.3 || freq_mhz > 100_000.0 || distance_m <= 0.0 {
    return None;
  }
  let p_mw = power_w * 1000.0;
  let g = linear_gain(gain_dbi);
  let mpe_c = mpe_limit_mw_cm2(freq_mhz, true);
  let mpe_u = mpe_limit_mw_cm2(freq_mhz, false);
  if mpe_c <= 0.0 || mpe_u <= 0.0 {
    return None;
  }
  let density = power_density_mw_cm2(power_w, gain_dbi, distance_m);
  let safe_c = (p_mw * g / (4.0 * PI * mpe_c)).sqrt() / 100.0;
  let safe_u = (p_mw * g / (4.0 * PI * mpe_u)).sqrt() / 100.0;
  Some(RfExposure {
    power_density: density,
    mpe_controlled: mpe_c,
    mpe_uncontrolled: mpe_u,
    safe_distance_controlled_m: safe_c,
    safe_distance_uncontrolled_m: safe_u,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mpe_hf_uncontrolled_uses_180_over_f_squared() {
    // 14 MHz：180 / 196 ≈ 0.918 mW/cm²。
    assert!((mpe_limit_mw_cm2(14.0, false) - 180.0 / 196.0).abs() < 1e-9);
    // 受控限值是公众限值的 5 倍（900/180）。
    assert!((mpe_limit_mw_cm2(14.0, true) - 900.0 / 196.0).abs() < 1e-9);
  }

  #[test]
  fn mpe_vhf_uhf_plateaus() {
    assert!((mpe_limit_mw_cm2(146.0, false) - 0.2).abs() < 1e-9);
    assert!((mpe_limit_mw_cm2(146.0, true) - 1.0).abs() < 1e-9);
    assert!((mpe_limit_mw_cm2(2000.0, false) - 1.0).abs() < 1e-9);
    assert!((mpe_limit_mw_cm2(2000.0, true) - 5.0).abs() < 1e-9);
  }

  #[test]
  fn out_of_range_returns_zero() {
    assert_eq!(mpe_limit_mw_cm2(0.1, false), 0.0);
    assert_eq!(mpe_limit_mw_cm2(200_000.0, true), 0.0);
  }

  #[test]
  fn power_density_inverse_square() {
    let d1 = power_density_mw_cm2(100.0, 0.0, 10.0);
    let d2 = power_density_mw_cm2(100.0, 0.0, 20.0);
    // 距离加倍，功率密度降为 1/4。
    assert!((d1 - 4.0 * d2).abs() < 1e-9);
  }

  #[test]
  fn assess_safe_distance_increases_with_power() {
    let low = assess(10.0, 14.0, 2.15, 10.0).unwrap();
    let high = assess(100.0, 14.0, 2.15, 10.0).unwrap();
    assert!(high.safe_distance_uncontrolled_m > low.safe_distance_uncontrolled_m);
    assert!(high.power_density > low.power_density);
  }

  #[test]
  fn assess_rejects_invalid_inputs() {
    assert!(assess(0.0, 14.0, 0.0, 10.0).is_none());
    assert!(assess(100.0, 0.1, 0.0, 10.0).is_none());
    assert!(assess(100.0, 14.0, 0.0, 0.0).is_none());
  }
}
