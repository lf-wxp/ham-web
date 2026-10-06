//! 射频暴露评估：按 FCC 47 CFR §1.1310 计算最大允许暴露（MPE）功率密度与最小安全距离。
//!
//! 采用远场近似：功率密度 S = P·G / (4π d²)。近场区（距离小于数个波长）实际场强
//! 可能更高，故本计算仅作合规自检参考，不能替代专业电磁环境评估。
//!
//! ## 适用范围（重要）
//!
//! 本模块只覆盖「已确定需要做常规评估」之后的 MPE 计算，**不包含** FCC 19-126
//! （2021-05-03 生效）新增的前置环节：
//!
//! - **豁免判定**：判据已由旧版 Part 97 的 PEP 门限表，改为 §1.1307(b)(3) 基于
//!   **ERP + 距离**的豁免公式（≤6 GHz 与 >6 GHz 两套门限）。应先判豁免，再谈评估。
//! - **时间平均**：可按发射占空比对功率做 source-based time-averaging
//!   （SSB 约 20%–40%、CW 约 40%、FM/RTTY/FT8 接近 100%）。本模块的 `assess`
//!   把入参功率直接当作平均功率，用于 SSB 时结果偏保守。
//! - **多发射源合计**：同一站点的多个发射机/天线需合并计算。
//! - **环境评估（EA）**：若人员可达区域超过 MPE 且无法通过降功率、增高、移位或
//!   限制接近来缓解，需按 §1.1307(a) / NEPA 向 FCC 提交环境评估。
//!
//! 平均时间仍为受控环境 6 分钟、非受控环境 30 分钟（§1.1310 注）。

use std::f64::consts::PI;

/// 非受控环境 MPE 表中 100 mW/cm² 段的上边界频率（MHz），见 47 CFR §1.1310
/// Table 1(B)。受控环境（Table 1(A)）对应的边界是 3.0 MHz。
pub const UNCONTROLLED_MF_CORNER_MHZ: f64 = 1.34;

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

/// FCC 47 CFR §1.1310 的 MPE 功率密度限值（mW/cm²）。
///
/// `controlled` 为 true 时返回职业/受控环境限值（Table 1(A)），否则为公众/非受控
/// 环境限值（Table 1(B)）。
///
/// 注意两条曲线的分段边界不同：受控环境 0.3–3.0 MHz 为 100 mW/cm²，而非受控环境
/// 只有 0.3–1.34 MHz 为 100 mW/cm²，1.34–30 MHz 走 `180 / f²`（1.8 MHz 处 55.6、
/// 3 MHz 处 20）。若把非受控也按 3.0 MHz 分段，会在 160m 波段把限值放宽到 5 倍，
/// 安全距离被低估约 2.2 倍。
///
/// 频率低于 0.3 MHz 或高于 100 GHz 返回 0（超出模型适用范围）。
#[must_use]
pub fn mpe_limit_mw_cm2(freq_mhz: f64, controlled: bool) -> f64 {
  let f = freq_mhz;
  match f {
    f if f < 0.3 => 0.0,
    // 受控 0.3–3.0 MHz 恒为 100；非受控仅到 1.34 MHz。
    f if f < 3.0 && (controlled || f < UNCONTROLLED_MF_CORNER_MHZ) => 100.0,
    f if f < 3.0 => 180.0 / (f * f),
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

/// 典型发射模式的功率占空比（用于 source-based time-averaging）。
///
/// FCC 19-126 允许在评估时按占空比对功率做时间平均。发射占空比越低，时间平均
/// 功率越小。取值是经验区间，实际应按自己的操作习惯选取。
pub const DUTY_CYCLES: &[(&str, f64)] = &[
  ("SSB 话音", 0.30),
  ("CW", 0.40),
  ("FM / 中继", 1.00),
  ("RTTY / 数字（连续）", 1.00),
  ("FT8 / FT4", 1.00),
  ("WSPR", 0.20),
];

/// 把峰包功率（PEP）换算为评估用的时间平均功率（瓦）。
///
/// `duty` 为占空比（0–1）。超出 [0,1] 时按端点裁剪。
#[must_use]
pub fn average_power_w(pep_w: f64, duty: f64) -> f64 {
  pep_w * duty.clamp(0.0, 1.0)
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
  fn mpe_uncontrolled_mf_corner_is_1_34_mhz() {
    // 非受控：0.3–1.34 MHz 恒为 100，1.34–30 MHz 走 180/f²。
    assert!((mpe_limit_mw_cm2(1.0, false) - 100.0).abs() < 1e-9);
    assert!((mpe_limit_mw_cm2(1.34, false) - 180.0 / (1.34 * 1.34)).abs() < 1e-9);
    // 160m 波段内不应再按 100 计算：1.8 MHz 应为 55.6、3 MHz 应为 20。
    assert!((mpe_limit_mw_cm2(1.8, false) - 55.555_555).abs() < 1e-3);
    assert!((mpe_limit_mw_cm2(3.0, false) - 20.0).abs() < 1e-9);
    // 受控环境一直到 3.0 MHz 都是 100。
    assert!((mpe_limit_mw_cm2(1.8, true) - 100.0).abs() < 1e-9);
    assert!((mpe_limit_mw_cm2(3.0, true) - 100.0).abs() < 1e-9);
  }

  #[test]
  fn average_power_scales_with_duty() {
    assert!((average_power_w(100.0, 0.3) - 30.0).abs() < 1e-9);
    // 越界占空比按端点裁剪。
    assert!((average_power_w(100.0, 5.0) - 100.0).abs() < 1e-9);
    assert_eq!(average_power_w(100.0, -1.0), 0.0);
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
