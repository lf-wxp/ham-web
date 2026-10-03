//! 直流供电线径与压降：按电流与长度估算导线电阻、压降与线径选择。

/// 铜电阻率（Ω·m，20°C）。
const COPPER_RHO: f64 = 1.68e-8;

/// 单根铜导线电阻（Ω）。
///
/// `len_m` 长度（米）、`diameter_mm` 导线直径（毫米）。
#[must_use]
pub fn copper_resistance_ohm(len_m: f64, diameter_mm: f64) -> f64 {
  if len_m <= 0.0 || diameter_mm <= 0.0 {
    return f64::NAN;
  }
  let area_m2 = std::f64::consts::PI * (diameter_mm / 1000.0 / 2.0).powi(2);
  COPPER_RHO * len_m / area_m2
}

/// 供电回路压降（V）。回路按「去 + 回」两倍长度计。
///
/// `amps` 电流（安）、`len_m` 单程长度（米）、`diameter_mm` 导线直径（毫米）。
#[must_use]
pub fn voltage_drop(amps: f64, len_m: f64, diameter_mm: f64) -> f64 {
  let r = copper_resistance_ohm(len_m, diameter_mm);
  if r.is_nan() {
    return f64::NAN;
  }
  2.0 * amps * r
}

/// 常用 AWG 线规：`(AWG, 直径 mm, 截面积 mm², 每 100m 电阻 Ω)`。
pub const AWG_TABLE: &[(&str, f64, f64, f64)] = &[
  ("10", 2.588, 5.26, 0.32),
  ("12", 2.053, 3.31, 0.51),
  ("14", 1.628, 2.08, 0.81),
  ("16", 1.291, 1.31, 1.28),
  ("18", 1.024, 0.823, 2.04),
  ("20", 0.812, 0.518, 3.24),
  ("22", 0.644, 0.326, 5.16),
  ("24", 0.511, 0.205, 8.20),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resistance_scales_with_length_and_area() {
    // 长度加倍 → 电阻加倍；直径减半 → 面积 1/4 → 电阻 4 倍。
    let r1 = copper_resistance_ohm(10.0, 1.0);
    let r2 = copper_resistance_ohm(20.0, 1.0);
    let r3 = copper_resistance_ohm(10.0, 0.5);
    assert!((r2 - 2.0 * r1).abs() < 1e-12);
    assert!((r3 - 4.0 * r1).abs() < 1e-12);
  }

  #[test]
  fn voltage_drop_is_round_trip() {
    let r = copper_resistance_ohm(10.0, 1.0);
    assert!((voltage_drop(2.0, 10.0, 1.0) - 2.0 * 2.0 * r).abs() < 1e-12);
  }

  #[test]
  fn invalid_inputs_return_nan() {
    assert!(copper_resistance_ohm(0.0, 1.0).is_nan());
    assert!(voltage_drop(1.0, 10.0, 0.0).is_nan());
  }

  #[test]
  fn awg_table_is_consistent() {
    for (i, &(_, dia, area, _)) in AWG_TABLE.iter().enumerate() {
      assert!(dia > 0.0 && area > 0.0);
      // 直径递减。
      if i > 0 {
        assert!(dia < AWG_TABLE[i - 1].1);
      }
    }
  }
}
