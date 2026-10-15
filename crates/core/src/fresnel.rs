//! 视距、地球曲率隆起与菲涅尔区净空：VHF / UHF / 微波点对点链路的基础估算。
//!
//! 公式都是教科书口径（d 为 km、h 为 m、f 为 MHz）：
//!
//! ```text
//! λ(m)            = c / f
//! r_n(m)          = √( n·λ·d₁·d₂ / (d₁+d₂) )          （d 换算成 m）
//! 地球曲率隆起(m)   = d₁·d₂ / (2·K·R)                    （R = 6371 km，K 为等效地球半径系数）
//! 无线电视距(km)    = √(2·K·R·h)                         （K = 4/3 时 ≈ 4.12·√h）
//! ```
//!
//! 建模边界：障碍物按**单个刃形峰**处理，高度与天线高度用同一海拔基准；
//! 不含植被、建筑与地面反射，也不算衍射附加损耗（只给「侵入第一菲涅尔区多深」）。

use crate::math::positive;

/// 真空光速（m/s）。
const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// 地球平均半径（km）。
pub const EARTH_RADIUS_KM: f64 = 6371.0;

/// 标准大气折射的等效地球半径系数 K = 4/3。
pub const K_STANDARD: f64 = 4.0 / 3.0;

/// 净空达到第一菲涅尔区半径的这个比例，损耗就接近自由空间（工程上的 60% 规则）。
pub const MIN_CLEARANCE_RATIO: f64 = 0.6;

/// 净空达到一整个第一菲涅尔区半径：视作完全不受遮挡。
pub const FULL_CLEARANCE_RATIO: f64 = 1.0;

/// 波长（m）；`freq_mhz` 必须 > 0。
#[must_use]
pub fn wavelength_m(freq_mhz: f64) -> Option<f64> {
  positive(freq_mhz).then(|| SPEED_OF_LIGHT_M_S / (freq_mhz * 1.0e6))
}

/// 第 `n` 菲涅尔区在距两端 `d1_km` / `d2_km` 处的半径（m）。`n` 为 0 与任一参数非法时返回 `None`。
#[must_use]
pub fn fresnel_radius_m(n: u32, freq_mhz: f64, d1_km: f64, d2_km: f64) -> Option<f64> {
  if n == 0 || !positive(d1_km) || !positive(d2_km) {
    return None;
  }
  let lambda = wavelength_m(freq_mhz)?;
  let (d1, d2) = (d1_km * 1000.0, d2_km * 1000.0);
  Some((f64::from(n) * lambda * d1 * d2 / (d1 + d2)).sqrt())
}

/// 地球曲率在距两端 `d1_km` / `d2_km` 处相对两端连线的隆起高度（m）。
/// 端点处（任一距离为 0）隆起为 0；负距离与非法 K 返回 `None`。
#[must_use]
pub fn earth_bulge_m(d1_km: f64, d2_km: f64, k: f64) -> Option<f64> {
  if !(d1_km.is_finite() && d1_km >= 0.0 && d2_km.is_finite() && d2_km >= 0.0 && positive(k)) {
    return None;
  }
  Some(d1_km * d2_km / (2.0 * k * EARTH_RADIUS_KM) * 1000.0)
}

/// 单个天线（离地 / 离海平面 `height_m`）的无线电地平线距离（km）。
#[must_use]
pub fn radio_horizon_km(height_m: f64, k: f64) -> Option<f64> {
  if !(height_m.is_finite() && height_m >= 0.0 && positive(k)) {
    return None;
  }
  Some((2.0 * k * EARTH_RADIUS_KM * 1000.0 * height_m).sqrt() / 1000.0)
}

/// 两个天线之间的最大视距（km）：两段地平线距离之和。
#[must_use]
pub fn line_of_sight_km(h1_m: f64, h2_m: f64, k: f64) -> Option<f64> {
  Some(radio_horizon_km(h1_m, k)? + radio_horizon_km(h2_m, k)?)
}

/// 净空判定档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
  /// 净空 ≥ 1 个第一菲涅尔区半径。
  Full,
  /// 净空 ≥ 60%：损耗接近自由空间。
  Sufficient,
  /// 视线通，但侵入第一菲涅尔区超过 40%。
  Partial,
  /// 障碍物顶（含曲率隆起）高过视线。
  Blocked,
}

impl Verdict {
  /// 按「净空 / 第一菲涅尔区半径」分档；`ratio < 0` 即被遮挡。
  #[must_use]
  pub fn from_ratio(ratio: f64) -> Self {
    if ratio >= FULL_CLEARANCE_RATIO {
      Self::Full
    } else if ratio >= MIN_CLEARANCE_RATIO {
      Self::Sufficient
    } else if ratio >= 0.0 {
      Self::Partial
    } else {
      Self::Blocked
    }
  }
}

/// 一条点对点路径与路径上的一个障碍物。高度全部用同一海拔基准（m）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathInput {
  pub freq_mhz: f64,
  /// A、B 两端的地面距离（km）。
  pub dist_km: f64,
  /// 两端天线离地高度（m），必须 ≥ 0。
  pub tx_height_m: f64,
  pub rx_height_m: f64,
  /// 等效地球半径系数；标准大气取 [`K_STANDARD`]。
  pub k: f64,
  /// 障碍物距 A 端的距离（km），必须落在 `(0, dist_km)` 内。
  pub obstacle_km: f64,
  /// 障碍物顶的海拔（m）。
  pub obstacle_height_m: f64,
}

/// [`analyze`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathReport {
  pub wavelength_m: f64,
  /// 障碍物位置处的第一菲涅尔区半径（m）。
  pub fresnel1_at_obstacle_m: f64,
  /// 路径中点的第一菲涅尔区半径（m，整条路径上最大）。
  pub fresnel1_mid_m: f64,
  /// 障碍物位置处的地球曲率隆起（m）。
  pub bulge_m: f64,
  /// 障碍物位置处两端天线连线的高度（m）。
  pub los_height_m: f64,
  /// 障碍物顶加上曲率隆起后的等效高度（m）。
  pub obstacle_effective_m: f64,
  /// 视线到障碍物等效顶的净空（m），负值即被遮挡。
  pub clearance_m: f64,
  /// 净空 / 第一菲涅尔区半径。
  pub clearance_ratio: f64,
  /// 相对 60% 规则的余量（m）：正值是还可以容忍的侵入深度，负值是还需抬高的高度。
  pub margin_60_m: f64,
  pub verdict: Verdict,
  /// 两端天线之间无障碍时的最大视距（km）。
  pub horizon_km: f64,
}

/// 分析障碍物对链路的影响；任一输入非法（含障碍物不在 A、B 之间）时返回 `None`。
#[must_use]
pub fn analyze(input: &PathInput) -> Option<PathReport> {
  let PathInput {
    freq_mhz,
    dist_km,
    tx_height_m,
    rx_height_m,
    k,
    obstacle_km,
    obstacle_height_m,
  } = *input;
  if !positive(dist_km) || !(obstacle_km > 0.0 && obstacle_km < dist_km) {
    return None;
  }
  if ![tx_height_m, rx_height_m, obstacle_height_m]
    .iter()
    .all(|v| v.is_finite())
  {
    return None;
  }
  // 天线离地高度为负（埋在地下）没有物理意义；障碍物顶的高度可以为负（低于海平面的地形合法）。
  if tx_height_m < 0.0 || rx_height_m < 0.0 {
    return None;
  }
  let d1 = obstacle_km;
  let d2 = dist_km - obstacle_km;
  let wavelength = wavelength_m(freq_mhz)?;
  let fresnel_at = fresnel_radius_m(1, freq_mhz, d1, d2)?;
  let fresnel_mid = fresnel_radius_m(1, freq_mhz, dist_km / 2.0, dist_km / 2.0)?;
  let bulge = earth_bulge_m(d1, d2, k)?;
  let los = tx_height_m + (rx_height_m - tx_height_m) * d1 / dist_km;
  let effective = obstacle_height_m + bulge;
  let clearance = los - effective;
  let ratio = clearance / fresnel_at;
  let horizon = line_of_sight_km(tx_height_m, rx_height_m, k)?;
  let report = PathReport {
    wavelength_m: wavelength,
    fresnel1_at_obstacle_m: fresnel_at,
    fresnel1_mid_m: fresnel_mid,
    bulge_m: bulge,
    los_height_m: los,
    obstacle_effective_m: effective,
    clearance_m: clearance,
    clearance_ratio: ratio,
    margin_60_m: clearance - MIN_CLEARANCE_RATIO * fresnel_at,
    verdict: Verdict::from_ratio(ratio),
    horizon_km: horizon,
  };
  [
    report.clearance_m,
    report.clearance_ratio,
    report.margin_60_m,
    report.horizon_km,
  ]
  .iter()
  .all(|v| v.is_finite())
  .then_some(report)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  /// 145 MHz、30 km、两端各 100 m、路径中点、地面海拔 0：只剩地球曲率。
  fn base() -> PathInput {
    PathInput {
      freq_mhz: 145.0,
      dist_km: 30.0,
      tx_height_m: 100.0,
      rx_height_m: 100.0,
      k: K_STANDARD,
      obstacle_km: 15.0,
      obstacle_height_m: 0.0,
    }
  }

  #[test]
  fn wavelength_matches_the_300_over_f_rule() {
    assert!(near(wavelength_m(100.0).unwrap(), 2.998, 1e-3));
    assert!(near(wavelength_m(145.0).unwrap(), 2.0675, 1e-3));
    assert_eq!(wavelength_m(0.0), None);
    assert_eq!(wavelength_m(f64::NAN), None);
  }

  #[test]
  fn first_zone_matches_the_textbook_coefficient() {
    // 教科书速算式 F1 = 17.32·√(d1·d2 / (f_GHz·D))；2.4 GHz、10 km、中点。
    let textbook = 17.32 * (5.0_f64 * 5.0 / (2.4 * 10.0)).sqrt();
    let r = fresnel_radius_m(1, 2400.0, 5.0, 5.0).unwrap();
    assert!((r - textbook).abs() / textbook < 2e-3, "{r} vs {textbook}");
    // 17.32 是 √300 的取整，用精确光速时是 17.31；差距在千分之一以内。
    assert!(near(r, 17.67, 0.02));
  }

  #[test]
  fn higher_zones_scale_with_root_n() {
    let r1 = fresnel_radius_m(1, 435.0, 12.0, 8.0).unwrap();
    let r3 = fresnel_radius_m(3, 435.0, 12.0, 8.0).unwrap();
    assert!(near(r3, r1 * 3.0_f64.sqrt(), 1e-9));
    assert_eq!(fresnel_radius_m(0, 435.0, 12.0, 8.0), None);
    assert_eq!(fresnel_radius_m(1, 435.0, 0.0, 8.0), None);
  }

  #[test]
  fn the_midpoint_zone_is_the_widest() {
    let mid = fresnel_radius_m(1, 145.0, 15.0, 15.0).unwrap();
    assert!(fresnel_radius_m(1, 145.0, 5.0, 25.0).unwrap() < mid);
    assert!(fresnel_radius_m(1, 145.0, 1.0, 29.0).unwrap() < mid);
  }

  #[test]
  fn earth_bulge_matches_the_classic_50_km_figures() {
    // 50 km 路径中点：K=1 约 49 m，K=4/3 约 36.8 m（地球曲率速算：d1·d2 / 12.74K）。
    assert!(near(earth_bulge_m(25.0, 25.0, 1.0).unwrap(), 49.05, 0.05));
    assert!(near(
      earth_bulge_m(25.0, 25.0, K_STANDARD).unwrap(),
      36.79,
      0.05
    ));
    assert_eq!(earth_bulge_m(0.0, 50.0, 1.0), Some(0.0));
    assert_eq!(earth_bulge_m(-1.0, 50.0, 1.0), None);
    assert_eq!(earth_bulge_m(25.0, 25.0, 0.0), None);
  }

  #[test]
  fn radio_horizon_matches_the_4_12_rule() {
    // K=1：3.57·√h；K=4/3：4.12·√h。
    assert!(near(radio_horizon_km(1.0, 1.0).unwrap(), 3.57, 0.005));
    assert!(near(
      radio_horizon_km(100.0, K_STANDARD).unwrap(),
      41.2,
      0.1
    ));
    // 两端 100 m 天线的最大视距是单端的两倍。
    assert!(near(
      line_of_sight_km(100.0, 100.0, K_STANDARD).unwrap(),
      82.5,
      0.2
    ));
    assert_eq!(radio_horizon_km(-1.0, 1.0), None);
  }

  #[test]
  fn flat_path_with_100m_masts_is_sufficient() {
    let r = analyze(&base()).unwrap();
    // 隆起 225/(12.742·4/3)=13.24 m；F1 = √(λ·7500)=124.5 m；净空 86.76 m = 0.697·F1。
    assert!(near(r.bulge_m, 13.24, 0.02));
    assert!(near(r.fresnel1_at_obstacle_m, 124.5, 0.1));
    assert!(near(r.fresnel1_mid_m, r.fresnel1_at_obstacle_m, 1e-9));
    assert!(near(r.clearance_m, 86.76, 0.05));
    assert!(near(r.clearance_ratio, 0.697, 0.002));
    assert_eq!(r.verdict, Verdict::Sufficient);
    assert!(r.margin_60_m > 0.0);
    assert!(near(r.horizon_km, 82.5, 0.2));
  }

  #[test]
  fn verdict_follows_obstacle_height() {
    let at = |h: f64| {
      analyze(&PathInput {
        obstacle_height_m: h,
        ..base()
      })
      .unwrap()
    };
    assert_eq!(at(0.0).verdict, Verdict::Sufficient);
    assert_eq!(at(40.0).verdict, Verdict::Partial);
    assert_eq!(at(200.0).verdict, Verdict::Blocked);
    assert!(at(200.0).clearance_m < 0.0);
    // 抬高天线到 300 m：整个第一菲涅尔区都通。
    let tall = analyze(&PathInput {
      tx_height_m: 300.0,
      rx_height_m: 300.0,
      ..base()
    })
    .unwrap();
    assert_eq!(tall.verdict, Verdict::Full);
  }

  #[test]
  fn sight_line_interpolates_between_unequal_masts() {
    let r = analyze(&PathInput {
      tx_height_m: 100.0,
      rx_height_m: 200.0,
      obstacle_km: 7.5,
      ..base()
    })
    .unwrap();
    assert!(near(r.los_height_m, 125.0, 1e-9));
  }

  #[test]
  fn margin_is_the_height_still_needed_or_spare() {
    let r = analyze(&PathInput {
      obstacle_height_m: 40.0,
      ..base()
    })
    .unwrap();
    // Partial：净空 < 60%·F1，余量为负，数值就是还要抬高的米数。
    assert!(r.margin_60_m < 0.0);
    let raised = analyze(&PathInput {
      obstacle_height_m: 40.0 + r.margin_60_m,
      ..base()
    })
    .unwrap();
    assert!(near(raised.clearance_ratio, MIN_CLEARANCE_RATIO, 1e-9));
  }

  #[test]
  fn rejects_invalid_inputs() {
    for bad in [
      PathInput {
        freq_mhz: 0.0,
        ..base()
      },
      PathInput {
        dist_km: 0.0,
        ..base()
      },
      PathInput { k: 0.0, ..base() },
      PathInput {
        obstacle_km: 0.0,
        ..base()
      },
      PathInput {
        obstacle_km: 30.0,
        ..base()
      },
      PathInput {
        obstacle_km: 31.0,
        ..base()
      },
      PathInput {
        tx_height_m: f64::NAN,
        ..base()
      },
      PathInput {
        obstacle_height_m: f64::INFINITY,
        ..base()
      },
      // 天线埋在地下没有物理意义。
      PathInput {
        tx_height_m: -1.0,
        ..base()
      },
      PathInput {
        rx_height_m: -1.0,
        ..base()
      },
    ] {
      assert_eq!(analyze(&bad), None, "{bad:?}");
    }
    // 障碍物顶低于海平面只是低于基准：高度合法，照常出结论。
    assert!(
      analyze(&PathInput {
        obstacle_height_m: -20.0,
        ..base()
      })
      .is_some()
    );
  }

  #[test]
  fn verdict_thresholds_are_inclusive_at_the_lower_bound() {
    assert_eq!(Verdict::from_ratio(1.0), Verdict::Full);
    assert_eq!(Verdict::from_ratio(0.6), Verdict::Sufficient);
    assert_eq!(Verdict::from_ratio(0.0), Verdict::Partial);
    assert_eq!(Verdict::from_ratio(-0.001), Verdict::Blocked);
  }
}
