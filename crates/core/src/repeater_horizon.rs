//! 中继台的无线电地平线：给定中继台坐标与天线离地高度，算出半径并取一圈地图上的点。
//!
//! 半径只调 [`crate::fresnel::radio_horizon_km`]（标准大气，K = 4/3），不另写视距公式；
//! 圆上的点用 [`crate::grid::circle_points`]。**天线高度数据里没有**，由用户填，不从备注猜。

use crate::fresnel::{K_STANDARD, radio_horizon_km};
use crate::grid::circle_points;

/// 天线离地高度的上限（m）：珠峰 8849 m，再往上不是地面台站。
pub const MAX_HEIGHT_M: f64 = 9000.0;

/// 圆周取点数：72 个点（每 5°）在 300 km 量级的圆上折线误差可以忽略。
pub const RING_POINTS: usize = 72;

/// 一个算好的无线电地平线。
#[derive(Debug, Clone, PartialEq)]
pub struct Horizon {
  /// 地平线距离（km）。
  pub radius_km: f64,
  /// 圆上的点 `(纬度, 经度)`，经度连续、可能越过 ±180°（见 [`circle_points`]）。
  pub ring: Vec<(f64, f64)>,
}

/// 算不出来的原因：界面按原因给不同的提示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizonError {
  /// 这条中继没有坐标（数据源缺失时服务端补 0 / 0）。
  NoCoordinates,
  /// 高度还没填。
  NoHeight,
  /// 高度不是 `(0, MAX_HEIGHT_M]` 内的数。
  BadHeight,
}

/// 坐标是否「像样」：服务端把缺失的经纬度写成 `0.0`，北纬 0° 东经 0°（几内亚湾）不会有中继。
#[must_use]
pub fn has_coordinates(lat: f64, lon: f64) -> bool {
  lat.is_finite()
    && lon.is_finite()
    && (-90.0..=90.0).contains(&lat)
    && (-180.0..=180.0).contains(&lon)
    && !(lat == 0.0 && lon == 0.0)
}

/// 解析天线高度（m）。
///
/// # Errors
/// 空串为 [`HorizonError::NoHeight`]；不是数、非正、超过 [`MAX_HEIGHT_M`] 为 [`HorizonError::BadHeight`]。
pub fn parse_height_m(text: &str) -> Result<f64, HorizonError> {
  let text = text.trim();
  if text.is_empty() {
    return Err(HorizonError::NoHeight);
  }
  match text.parse::<f64>() {
    Ok(h) if h.is_finite() && h > 0.0 && h <= MAX_HEIGHT_M => Ok(h),
    _ => Err(HorizonError::BadHeight),
  }
}

/// 以 `(lat, lon)` 为中心、天线高度为 `height_text`（m）的无线电地平线。
///
/// # Errors
/// 坐标缺失优先报 [`HorizonError::NoCoordinates`]，其次是高度的两种错误。
pub fn horizon(lat: f64, lon: f64, height_text: &str) -> Result<Horizon, HorizonError> {
  if !has_coordinates(lat, lon) {
    return Err(HorizonError::NoCoordinates);
  }
  let height = parse_height_m(height_text)?;
  let radius_km = radio_horizon_km(height, K_STANDARD).ok_or(HorizonError::BadHeight)?;
  let ring = circle_points(lat, lon, radius_km, RING_POINTS);
  Ok(Horizon { radius_km, ring })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::grid::distance_bearing;

  #[test]
  fn radius_matches_the_4_12_root_h_rule_of_thumb() {
    // 业余无线电里通行的口算：标准大气下地平线 ≈ 4.12·√h（h 单位 m，结果 km）。
    // 与被测代码无关的期望值；4.12 是四舍五入的系数，给 0.1 km 容差。
    for h in [2.0_f64, 30.0, 120.0, 1500.0] {
      let got = horizon(31.2, 121.5, &h.to_string()).unwrap().radius_km;
      let rule = 4.12 * h.sqrt();
      assert!(
        (got - rule).abs() < 0.1 + rule * 0.002,
        "h={h}: {got} vs {rule}"
      );
    }
  }

  #[test]
  fn the_ring_is_a_circle_of_that_radius_around_the_repeater() {
    let h = horizon(31.2, 121.5, "120").unwrap();
    assert_eq!(h.ring.len(), RING_POINTS);
    for &(lat, lon) in &h.ring {
      let (d, _) = distance_bearing(31.2, 121.5, lat, lon);
      assert!((d - h.radius_km).abs() < 1e-6, "{d} vs {}", h.radius_km);
    }
  }

  #[test]
  fn a_taller_antenna_reaches_further() {
    let low = horizon(31.2, 121.5, "10").unwrap().radius_km;
    let high = horizon(31.2, 121.5, "40").unwrap().radius_km;
    // √4 = 2：高度翻两番、半径翻一番。
    assert!((high / low - 2.0).abs() < 1e-9, "{low} → {high}");
  }

  #[test]
  fn height_text_is_parsed_strictly() {
    assert_eq!(parse_height_m(" 30 "), Ok(30.0));
    assert_eq!(parse_height_m("0.5"), Ok(0.5));
    assert_eq!(parse_height_m("9000"), Ok(9000.0));
    assert_eq!(parse_height_m(""), Err(HorizonError::NoHeight));
    assert_eq!(parse_height_m("   "), Err(HorizonError::NoHeight));
    for bad in [
      "abc", "0", "-5", "9000.1", "NaN", "inf", "-inf", "3 m", "1,5",
    ] {
      assert_eq!(parse_height_m(bad), Err(HorizonError::BadHeight), "{bad:?}");
    }
  }

  #[test]
  fn missing_coordinates_win_over_height_problems() {
    // 服务端把缺失写成 0 / 0：不能把一个不存在的位置画到几内亚湾。
    assert_eq!(horizon(0.0, 0.0, "30"), Err(HorizonError::NoCoordinates));
    assert_eq!(horizon(0.0, 0.0, ""), Err(HorizonError::NoCoordinates));
    assert_eq!(
      horizon(f64::NAN, 10.0, "30"),
      Err(HorizonError::NoCoordinates)
    );
    assert_eq!(horizon(95.0, 10.0, "30"), Err(HorizonError::NoCoordinates));
    assert_eq!(horizon(10.0, 181.0, "30"), Err(HorizonError::NoCoordinates));
    // 只有一个分量为 0 是真实位置（赤道上、本初子午线上）。
    assert!(has_coordinates(0.0, 36.8));
    assert!(has_coordinates(51.5, 0.0));
  }

  #[test]
  fn height_errors_are_reported_once_coordinates_are_fine() {
    assert_eq!(horizon(31.2, 121.5, ""), Err(HorizonError::NoHeight));
    assert_eq!(horizon(31.2, 121.5, "x"), Err(HorizonError::BadHeight));
  }
}
