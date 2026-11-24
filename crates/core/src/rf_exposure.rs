//! 射频暴露评估：按 FCC 47 CFR §1.1310 计算最大允许暴露（MPE）功率密度与最小安全距离，
//! 另有一条只认 30–3000 MHz 的中国 GB 8702-2014 公众曝露口径（[`assess_gb8702_public`]）。
//!
//! **两套口径不能互相代入**：GB 8702 的公众限值在 30–3000 MHz 恒为 0.4 W/m²，而 FCC 非受控限值
//! 随频率抬高 —— 30–300 MHz 是 2 W/m²（5 倍，安全距离相差 √5 ≈ 2.24 倍），435 MHz 约 2.9 W/m²
//! （7.25 倍），1.5 GHz 以上 10 W/m²（25 倍）。国内合规判定看 GB 8702，FCC 只作对照。
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

/// GB 8702-2014 公众曝露限值（30–3000 MHz）的等效平面波功率密度（W/m²）。
///
/// 标准同一行还写了电场强度 12 V/m；12² / 377 ≈ 0.38，标准取整写作 0.4，这里照标准写的值，
/// 不自己用 12 V/m 反算（会差 4.5%）。与 [`crate::safety::EXPOSURE_LIMITS`] 的正文同值，
/// 由 `knowledge_consistency` 对拍。
pub const GB8702_PUBLIC_W_M2: f64 = 0.4;

/// [`GB8702_PUBLIC_W_M2`] 适用的频率范围（MHz，含两端）。
///
/// 本站只收录这一行：GB 8702 在 30 MHz 以下（HF）与 3 GHz 以上另有分段限值，没有逐字核对过的
/// 限值表之前不外推 —— 把 0.4 W/m² 用到 HF 会得出一个看起来像真的、实际错的距离。
pub const GB8702_PUBLIC_RANGE_MHZ: (f64, f64) = (30.0, 3000.0);

/// 按 GB 8702-2014 评估公众曝露的结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gb8702Public {
  /// 给定距离处的功率密度（W/m²，远场近似）。
  pub power_density_w_m2: f64,
  /// 公众限值（W/m²），即 [`GB8702_PUBLIC_W_M2`]。
  pub limit_w_m2: f64,
  /// 满足公众限值所需的最小距离（米）。
  pub safe_distance_m: f64,
}

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

/// 按 GB 8702-2014 评估**公众**曝露：功率密度、限值与最小安全距离（远场近似）。
///
/// `power_w` 按 6 分钟平均功率处理 —— SSB / CW 的峰包功率要先折算（见 `duty_power`）。
/// 频率不在 [`GB8702_PUBLIC_RANGE_MHZ`] 内、任一入参非正或非有限、结果溢出时返回 `None`，
/// 页面据此说明「不给国内结论」而不是退回 FCC 的数。
///
/// 只覆盖公众曝露：职业曝露限值、豁免条件、多部发射机相加与近场修正都没有收录。
#[must_use]
pub fn assess_gb8702_public(
  power_w: f64,
  freq_mhz: f64,
  gain_dbi: f64,
  distance_m: f64,
) -> Option<Gb8702Public> {
  let (lo, hi) = GB8702_PUBLIC_RANGE_MHZ;
  let valid = [power_w, freq_mhz, gain_dbi, distance_m]
    .iter()
    .all(|v| v.is_finite())
    && power_w > 0.0
    && distance_m > 0.0
    && (lo..=hi).contains(&freq_mhz);
  if !valid {
    return None;
  }
  let eirp_w = power_w * linear_gain(gain_dbi);
  let result = Gb8702Public {
    power_density_w_m2: eirp_w / (4.0 * PI * distance_m * distance_m),
    limit_w_m2: GB8702_PUBLIC_W_M2,
    safe_distance_m: (eirp_w / (4.0 * PI * GB8702_PUBLIC_W_M2)).sqrt(),
  };
  (result.power_density_w_m2.is_finite() && result.safe_distance_m.is_finite()).then_some(result)
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

  /// 期望值来自独立脚本：S = P·G/(4πd²)，d = √(P·G/(4π·0.4))（不复用本模块的公式）。
  #[test]
  fn gb8702_matches_independent_anchors() {
    let r = assess_gb8702_public(100.0, 145.0, 0.0, 10.0).unwrap();
    assert!((r.power_density_w_m2 - 0.079_577).abs() < 1e-6);
    assert!((r.safe_distance_m - 4.460_310).abs() < 1e-6);
    assert_eq!(r.limit_w_m2, 0.4);
    // 11.15 dBi 的定向天线：同样 100 W，安全距离按 √G 放大。
    let r = assess_gb8702_public(100.0, 145.0, 11.15, 10.0).unwrap();
    assert!((r.safe_distance_m - 16.101_453).abs() < 1e-5);
  }

  /// 安全距离代回去，功率密度正好等于限值；这一步抓得住「距离公式与密度公式不互逆」。
  #[test]
  fn gb8702_safe_distance_round_trips_to_the_limit() {
    for (p, f, g) in [
      (5.0, 435.0, 2.15),
      (100.0, 145.0, 0.0),
      (1500.0, 1296.0, 18.0),
    ] {
      let r = assess_gb8702_public(p, f, g, 10.0).unwrap();
      let at_safe = assess_gb8702_public(p, f, g, r.safe_distance_m).unwrap();
      assert!((at_safe.power_density_w_m2 - GB8702_PUBLIC_W_M2).abs() < 1e-12);
    }
  }

  /// 「差 5 倍」只在 30–300 MHz 成立（FCC 非受控 0.2 mW/cm² = 2 W/m²，距离比正好 √5）；
  /// 300 MHz 以上 FCC 的限值随频率抬高，差距更大，不能把 5 倍当成 30–3000 MHz 的通则。
  /// 期望的倍数由独立脚本按 FCC 分段算出（435 MHz 为 2.9 W/m²，即 7.25 倍）。
  #[test]
  fn gb8702_vs_fcc_distance_ratio_depends_on_the_band() {
    let ratio = |f: f64| {
      let gb = assess_gb8702_public(100.0, f, 2.15, 10.0).unwrap();
      let fcc = assess(100.0, f, 2.15, 10.0).unwrap();
      gb.safe_distance_m / fcc.safe_distance_uncontrolled_m
    };
    assert!((ratio(146.0) - 5f64.sqrt()).abs() < 1e-9);
    assert!((ratio(435.0) - 7.25f64.sqrt()).abs() < 1e-9);
    assert!((ratio(2400.0) - 25f64.sqrt()).abs() < 1e-9);
  }

  /// 频段边界含两端；段外（HF、3 GHz 以上）一律 `None`，不外推。
  #[test]
  fn gb8702_refuses_frequencies_outside_30_to_3000_mhz() {
    assert!(assess_gb8702_public(100.0, 30.0, 0.0, 10.0).is_some());
    assert!(assess_gb8702_public(100.0, 3000.0, 0.0, 10.0).is_some());
    for f in [14.0, 29.999, 3000.001, 5800.0, 0.0, -1.0] {
      assert!(assess_gb8702_public(100.0, f, 0.0, 10.0).is_none(), "{f}");
    }
  }

  #[test]
  fn gb8702_rejects_invalid_and_overflowing_inputs() {
    assert!(assess_gb8702_public(0.0, 145.0, 0.0, 10.0).is_none());
    assert!(assess_gb8702_public(-5.0, 145.0, 0.0, 10.0).is_none());
    assert!(assess_gb8702_public(100.0, 145.0, 0.0, 0.0).is_none());
    assert!(assess_gb8702_public(100.0, 145.0, 0.0, -3.0).is_none());
    assert!(assess_gb8702_public(f64::NAN, 145.0, 0.0, 10.0).is_none());
    assert!(assess_gb8702_public(100.0, f64::NAN, 0.0, 10.0).is_none());
    assert!(assess_gb8702_public(100.0, 145.0, f64::INFINITY, 10.0).is_none());
    assert!(assess_gb8702_public(f64::INFINITY, 145.0, 0.0, 10.0).is_none());
    // 增益大到线性值溢出：不返回一个 inf 的「安全距离」。
    assert!(assess_gb8702_public(100.0, 145.0, 5000.0, 10.0).is_none());
  }
}
