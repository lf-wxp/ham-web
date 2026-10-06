//! 传输线特性阻抗：同轴、平行双线、微带线的特性阻抗估算。
//!
//! 速度因子与损耗数据见 [`crate::feedline::FEEDLINE_SPECS`]，
//! 线缆内波长见 [`crate::feedline::wavelength_in_line_m`]。

/// 同轴线特性阻抗（Ω）。
///
/// `er` 为介质相对介电常数，`d_outer` 为外导体内径，`d_inner` 为内导体外径
/// （两者单位一致即可，常用毫米）。Z₀ = 138/√εr · lg(D/d)。
#[must_use]
pub fn coax_z0(er: f64, d_outer: f64, d_inner: f64) -> f64 {
  if er <= 0.0 || d_outer <= d_inner || d_inner <= 0.0 {
    return f64::NAN;
  }
  138.0 / er.sqrt() * (d_outer / d_inner).log10()
}

/// 平行双线特性阻抗（Ω）。
///
/// `er` 为介质相对介电常数，`spacing` 为两导线中心距，`diameter` 为导线直径。
/// Z₀ = 276/√εr · lg(2S/d)。
#[must_use]
pub fn twin_lead_z0(er: f64, spacing: f64, diameter: f64) -> f64 {
  if er <= 0.0 || spacing <= diameter || diameter <= 0.0 {
    return f64::NAN;
  }
  276.0 / er.sqrt() * (2.0 * spacing / diameter).log10()
}

/// 微带线特性阻抗（Ω），闭式近似。
///
/// `er` 为基板介电常数，`width_mm` 为走线宽度，`height_mm` 为介质厚度。
#[must_use]
pub fn microstrip_z0(er: f64, width_mm: f64, height_mm: f64) -> f64 {
  if er <= 1.0 || width_mm <= 0.0 || height_mm <= 0.0 {
    return f64::NAN;
  }
  let w_h = width_mm / height_mm;
  let ee = (er + 1.0) / 2.0 + (er - 1.0) / 2.0 / (1.0 + 12.0 / w_h).sqrt();
  if w_h <= 1.0 {
    60.0 / ee.sqrt() * (8.0 / w_h + w_h / 4.0).ln()
  } else {
    let denom = w_h + 1.393 + 0.667 * (w_h + 1.444).ln();
    120.0 * std::f64::consts::PI / (ee.sqrt() * denom)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rg58_like_coax_is_about_50_ohm() {
    // εr=2.25（聚乙烯）、D/d=3.5 → 138/1.5·lg3.5 ≈ 50.1 Ω。
    let z = coax_z0(2.25, 3.5, 1.0);
    assert!((z - 50.0).abs() < 1.0, "z={z}");
  }

  #[test]
  fn air_coax_75_ohm() {
    // εr≈1、D/d≈3.5 → 138·lg3.5 ≈ 75.1 Ω。
    let z = coax_z0(1.0, 3.5, 1.0);
    assert!((z - 75.0).abs() < 2.0, "z={z}");
  }

  #[test]
  fn twin_lead_300_ohm() {
    // 常见 300Ω 平行馈线：εr≈1，S/d ≈ 6.1 → 276·lg12.2 ≈ 299。
    let z = twin_lead_z0(1.0, 6.1, 1.0);
    assert!((z - 300.0).abs() < 15.0, "z={z}");
  }

  #[test]
  fn microstrip_50_ohm_fr4() {
    // FR4（εr≈4.4）上 50Ω 微带线 W/h ≈ 1.9。
    let z = microstrip_z0(4.4, 1.9, 1.0);
    assert!((z - 50.0).abs() < 3.0, "z={z}");
  }

  #[test]
  fn invalid_inputs_return_nan() {
    assert!(coax_z0(2.25, 1.0, 3.5).is_nan());
    assert!(twin_lead_z0(2.25, 0.5, 1.0).is_nan());
    assert!(microstrip_z0(1.0, 1.0, 1.0).is_nan());
  }
}
