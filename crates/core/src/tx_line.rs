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

/// 微带线有效介电常数 εeff。
///
/// `er` 为基板介电常数，`width_mm` 为走线宽度，`height_mm` 为介质厚度。
#[must_use]
pub fn microstrip_eeff(er: f64, width_mm: f64, height_mm: f64) -> f64 {
  if er <= 1.0 || width_mm <= 0.0 || height_mm <= 0.0 {
    return f64::NAN;
  }
  let w_h = width_mm / height_mm;
  (er + 1.0) / 2.0 + (er - 1.0) / 2.0 / (1.0 + 12.0 / w_h).sqrt()
}

/// 由 εeff 与 `W/h` 求微带线特性阻抗（Ω）。
fn microstrip_z0_from(ee: f64, w_h: f64) -> f64 {
  if w_h <= 1.0 {
    60.0 / ee.sqrt() * (8.0 / w_h + w_h / 4.0).ln()
  } else {
    let denom = w_h + 1.393 + 0.667 * (w_h + 1.444).ln();
    120.0 * std::f64::consts::PI / (ee.sqrt() * denom)
  }
}

/// 微带线特性阻抗（Ω），闭式近似。
#[must_use]
pub fn microstrip_z0(er: f64, width_mm: f64, height_mm: f64) -> f64 {
  let ee = microstrip_eeff(er, width_mm, height_mm);
  if ee.is_nan() {
    return f64::NAN;
  }
  microstrip_z0_from(ee, width_mm / height_mm)
}

/// 微带线 (`Z0`, `εeff`) 一次算完。
///
/// 界面要同时显示这两个值；分开调两个函数会把 εeff 求两遍。非法输入时两者都是 `NaN`。
#[must_use]
pub fn microstrip(er: f64, width_mm: f64, height_mm: f64) -> (f64, f64) {
  let ee = microstrip_eeff(er, width_mm, height_mm);
  if ee.is_nan() {
    return (f64::NAN, f64::NAN);
  }
  (microstrip_z0_from(ee, width_mm / height_mm), ee)
}

/// 对称带状线特性阻抗（Ω），闭式近似（Cohn，零厚度）。
///
/// `er` 为介质介电常数，`width_mm` 为走线宽度，`spacing_mm` 为上下地平面间距（b）。
/// 带状线介质均匀，εeff = εr，无需单独换算。
#[must_use]
pub fn stripline_z0(er: f64, width_mm: f64, spacing_mm: f64) -> f64 {
  if er <= 1.0 || width_mm <= 0.0 || spacing_mm <= 0.0 {
    return f64::NAN;
  }
  let w_b = width_mm / spacing_mm;
  if w_b >= 0.35 {
    30.0 * std::f64::consts::PI / er.sqrt() / (w_b + 0.441)
  } else {
    60.0 / er.sqrt() * (8.0 * spacing_mm / (std::f64::consts::PI * width_mm)).ln()
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
  fn microstrip_pair_agrees_with_single_value_helpers() {
    // 合成函数只是省一次 εeff：两个值必须与分别调用时逐位相同。
    let (z, ee) = microstrip(4.4, 1.9, 1.0);
    assert_eq!(z, microstrip_z0(4.4, 1.9, 1.0));
    assert_eq!(ee, microstrip_eeff(4.4, 1.9, 1.0));
    let (z, ee) = microstrip(1.0, 1.9, 1.0);
    assert!(z.is_nan() && ee.is_nan(), "εr ≤ 1 应整对拒绝");
  }

  #[test]
  fn microstrip_eeff_between_1_and_er() {
    // εeff 介于 1 与 εr 之间；FR4、W/h=1.9 → εeff ≈ 3.33。
    let ee = microstrip_eeff(4.4, 1.9, 1.0);
    assert!((ee - 3.33).abs() < 0.05, "ee={ee}");
    assert!(1.0 < ee && ee < 4.4);
    // 极宽走线趋近 εr，极窄走线趋近 (εr+1)/2。
    assert!((microstrip_eeff(4.4, 1000.0, 1.0) - 4.4).abs() < 0.05);
    assert!((microstrip_eeff(4.4, 0.01, 1.0) - 2.7).abs() < 0.05);
  }

  #[test]
  fn stripline_wide_and_narrow_agree_at_boundary() {
    // W/b ≥ 0.35 用宽公式、< 0.35 用窄公式，边界处应连续。
    let wide = stripline_z0(4.4, 0.36, 1.0);
    let narrow = stripline_z0(4.4, 0.34, 1.0);
    assert!((wide - narrow).abs() < 3.0, "wide={wide} narrow={narrow}");
    // FR4、W/b = 0.5 → Z0 ≈ 47.7 Ω。
    let z = stripline_z0(4.4, 0.5, 1.0);
    assert!((z - 47.7).abs() < 0.5, "z={z}");
  }

  #[test]
  fn stripline_invalid_inputs_return_nan() {
    assert!(stripline_z0(1.0, 1.0, 1.0).is_nan());
    assert!(stripline_z0(4.4, 0.0, 1.0).is_nan());
    assert!(stripline_z0(4.4, 1.0, 0.0).is_nan());
  }

  #[test]
  fn invalid_inputs_return_nan() {
    assert!(coax_z0(2.25, 1.0, 3.5).is_nan());
    assert!(twin_lead_z0(2.25, 0.5, 1.0).is_nan());
    assert!(microstrip_z0(1.0, 1.0, 1.0).is_nan());
  }
}
