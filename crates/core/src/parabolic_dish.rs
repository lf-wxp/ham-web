//! 抛物面天线：由口径、频率与效率算增益、波束宽度，由 f/D 或实测凹陷深度算焦点位置。
//!
//! 公式（`D` 口径、`λ` 波长、`η` 孔径效率、`f` 焦距、`d` 凹陷深度，同一长度单位）：
//!
//! ```text
//! 增益        G = η·(π·D/λ)²           G(dBi) = 10·log10(G)
//! 有效面积    A_e = η·π·D²/4           （G = 4π·A_e/λ² 与上式等价）
//! 3 dB 波束   θ ≈ 70·λ/D  度            经验近似，见下
//! 凹陷深度    d = D² / (16·f)           f/D = D / (16·d)
//! 对焦点张角  Ψ = 4·atan(D / (4·f))     （焦点看向反射面边缘的全角）
//! 焦点位置    在轴线上距顶点 f 处；相对口径平面 f − d（> 0 在碗外，< 0 在碗内）
//! ```
//!
//! 边界（**读之前先知道**）：
//! - **效率 η 是输入，不是算出来的**。它包含馈源照射（溢出与渐削）、表面精度、遮挡与失配损耗，
//!   实际取决于馈源与制作质量；默认 55% 只是业余自制 / 成品小口径面的常见量级，
//!   不是任何具体天线的保证值；
//! - 波束宽度 `70·λ/D` 是常用近似（系数随照射渐削在 65–75 之间浮动），不是实测值；
//! - 表面精度（均方根误差）、馈源与支架遮挡、偏馈面、双反射面都**没有**单独建模，只能折进 η；
//! - 口径只有几个波长时（见 [`SMALL_APERTURE_WAVELENGTHS`]）增益公式不再可靠
//!   （远场与口径照射的假设都站不住），结果会带 `small_aperture` 标记，仅作量级参考；
//! - 只处理旋转抛物面（含前馈与深碗），长度单位统一用米。

use crate::fresnel::wavelength_m;
use crate::math::positive;

/// 默认孔径效率（55%）。
pub const DEFAULT_EFFICIENCY: f64 = 0.55;
/// 3 dB 波束宽度近似的系数（度 · 口径波长数）。
pub const BEAMWIDTH_FACTOR_DEG: f64 = 70.0;
/// 口径小于这么多个波长时给出「增益公式不可靠」标记（经验界限，不是理论界限）。
pub const SMALL_APERTURE_WAVELENGTHS: f64 = 3.0;

/// 计算输入。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DishInput {
  pub freq_mhz: f64,
  /// 口径（m）。
  pub diameter_m: f64,
  /// 孔径效率，`(0, 1]`。
  pub efficiency: f64,
  /// 焦径比 f/D，正数。
  pub f_over_d: f64,
}

/// 计算结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DishReport {
  pub wavelength_m: f64,
  /// 口径是多少个波长。
  pub diameter_wavelengths: f64,
  pub gain_linear: f64,
  pub gain_dbi: f64,
  /// 有效面积（m²）。
  pub effective_area_m2: f64,
  /// 3 dB 波束宽度近似（度）。
  pub beamwidth_deg: f64,
  /// 焦距（m）。
  pub focal_length_m: f64,
  /// 凹陷深度（m）。
  pub depth_m: f64,
  /// 焦点相对口径平面的位置（m）：正数在碗外（朝着发射方向），负数在碗内，0 恰在口径平面上。
  pub focus_above_rim_m: f64,
  /// 从焦点看反射面的全张角（度）；f/D = 0.25 时为 180°。
  pub subtended_angle_deg: f64,
  /// 口径不足 [`SMALL_APERTURE_WAVELENGTHS`] 个波长：增益公式不可靠。
  pub small_aperture: bool,
}

/// 焦距（m）与凹陷深度（m）：已知口径与 f/D。
fn geometry(diameter_m: f64, f_over_d: f64) -> (f64, f64) {
  let f = f_over_d * diameter_m;
  (f, diameter_m * diameter_m / (16.0 * f))
}

/// 对焦点的全张角（度）：`4·atan(D / (4f)) = 4·atan(1 / (4·f/D))`。
fn subtended_angle_deg(f_over_d: f64) -> f64 {
  4.0 * (1.0 / (4.0 * f_over_d)).atan().to_degrees()
}

/// 计算增益、波束宽度与焦点几何；任一输入不合法返回 `None`。
#[must_use]
pub fn analyze(input: &DishInput) -> Option<DishReport> {
  let eta = input.efficiency;
  if !(positive(input.diameter_m)
    && positive(input.f_over_d)
    && eta.is_finite()
    && eta > 0.0
    && eta <= 1.0)
  {
    return None;
  }
  let lambda = wavelength_m(input.freq_mhz)?;
  let d_over_l = input.diameter_m / lambda;
  let gain_linear = eta * (std::f64::consts::PI * d_over_l).powi(2);
  let (focal, depth) = geometry(input.diameter_m, input.f_over_d);
  Some(DishReport {
    wavelength_m: lambda,
    diameter_wavelengths: d_over_l,
    gain_linear,
    gain_dbi: 10.0 * gain_linear.log10(),
    effective_area_m2: eta * std::f64::consts::PI * input.diameter_m * input.diameter_m / 4.0,
    beamwidth_deg: BEAMWIDTH_FACTOR_DEG / d_over_l,
    focal_length_m: focal,
    depth_m: depth,
    focus_above_rim_m: focal - depth,
    subtended_angle_deg: subtended_angle_deg(input.f_over_d),
    small_aperture: d_over_l < SMALL_APERTURE_WAVELENGTHS,
  })
}

/// 由实测口径与凹陷深度反推焦距与 f/D：`f = D² / (16·d)`。
///
/// 业余圈量未知抛物面焦点的常用办法：用直尺横跨口径、量中心到直尺的距离。
/// 两个量都必须是正数。
#[must_use]
pub fn focus_from_depth(diameter_m: f64, depth_m: f64) -> Option<(f64, f64)> {
  (positive(diameter_m) && positive(depth_m)).then(|| {
    let f = diameter_m * diameter_m / (16.0 * depth_m);
    (f, f / diameter_m)
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  fn input(freq_mhz: f64, d: f64, eta: f64, fd: f64) -> DishInput {
    DishInput {
      freq_mhz,
      diameter_m: d,
      efficiency: eta,
      f_over_d: fd,
    }
  }

  // 以下对拍值均由与实现无关的 Python 脚本独立算出（c = 299 792 458 m/s）。
  #[test]
  fn gain_matches_independently_computed_values() {
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.4)).unwrap();
    assert!(near(r.wavelength_m, 0.029_979_245_8, 1e-12));
    assert!(near(r.gain_linear, 6039.7787, 1e-3), "{}", r.gain_linear);
    assert!(near(r.gain_dbi, 37.8102, 1e-3), "{}", r.gain_dbi);

    let r = analyze(&input(10_368.0, 3.0, 0.55, 0.4)).unwrap();
    assert!(near(r.gain_dbi, 47.6665, 1e-3), "{}", r.gain_dbi);

    let r = analyze(&input(5_760.0, 0.6, 0.5, 0.4)).unwrap();
    assert!(near(r.gain_dbi, 28.1678, 1e-3), "{}", r.gain_dbi);

    // 效率 100%：理想口径增益。
    let r = analyze(&input(10_000.0, 1.0, 1.0, 0.4)).unwrap();
    assert!(near(r.gain_dbi, 40.4066, 1e-3), "{}", r.gain_dbi);
  }

  #[test]
  fn gain_agrees_with_the_common_rule_of_thumb() {
    // 常用速算：G(dBi) ≈ 17.8 + 20·log10(D/m) + 20·log10(f/GHz)（55% 效率）。
    for (f_mhz, d) in [
      (1296.0, 3.0),
      (2400.0, 0.9),
      (5760.0, 1.2),
      (10_368.0, 0.6),
      (24_048.0, 0.3),
    ] {
      let r = analyze(&input(f_mhz, d, 0.55, 0.4)).unwrap();
      let rule = 17.8 + 20.0 * d.log10() + 20.0 * (f_mhz / 1000.0).log10();
      assert!(
        near(r.gain_dbi, rule, 0.05),
        "{f_mhz} MHz {d} m: {} vs {rule}",
        r.gain_dbi
      );
    }
  }

  #[test]
  fn gain_scales_the_way_physics_says() {
    let base = analyze(&input(10_000.0, 1.0, 0.55, 0.4)).unwrap();
    let double_d = analyze(&input(10_000.0, 2.0, 0.55, 0.4)).unwrap();
    let double_f = analyze(&input(20_000.0, 1.0, 0.55, 0.4)).unwrap();
    // 口径或频率翻倍：增益 +6.0206 dB（面积 ×4）。
    assert!(near(double_d.gain_dbi - base.gain_dbi, 6.0206, 1e-3));
    assert!(near(double_f.gain_dbi - base.gain_dbi, 6.0206, 1e-3));
    // 效率每降一半：−3.0103 dB。
    let half_eta = analyze(&input(10_000.0, 1.0, 0.275, 0.4)).unwrap();
    assert!(near(base.gain_dbi - half_eta.gain_dbi, 3.0103, 1e-3));
  }

  #[test]
  fn gain_equals_four_pi_effective_area_over_lambda_squared() {
    let r = analyze(&input(10_368.0, 1.2, 0.6, 0.35)).unwrap();
    let from_area =
      4.0 * std::f64::consts::PI * r.effective_area_m2 / (r.wavelength_m * r.wavelength_m);
    assert!(near(r.gain_linear / from_area, 1.0, 1e-12));
    // D = 1 m、55%：A_e = 0.55·π/4 = 0.4320 m²。
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.4)).unwrap();
    assert!(
      near(r.effective_area_m2, 0.431_969, 1e-6),
      "{}",
      r.effective_area_m2
    );
  }

  #[test]
  fn beamwidth_is_70_lambda_over_d() {
    assert!(near(
      analyze(&input(10_000.0, 1.0, 0.55, 0.4))
        .unwrap()
        .beamwidth_deg,
      2.0985,
      1e-3
    ));
    assert!(near(
      analyze(&input(10_368.0, 3.0, 0.55, 0.4))
        .unwrap()
        .beamwidth_deg,
      0.6747,
      1e-3
    ));
    assert!(near(
      analyze(&input(5_760.0, 0.6, 0.5, 0.4))
        .unwrap()
        .beamwidth_deg,
      6.0722,
      1e-3
    ));
    // 口径翻倍波束减半。
    let a = analyze(&input(10_000.0, 1.0, 0.55, 0.4))
      .unwrap()
      .beamwidth_deg;
    let b = analyze(&input(10_000.0, 2.0, 0.55, 0.4))
      .unwrap()
      .beamwidth_deg;
    assert!(near(a / b, 2.0, 1e-12));
  }

  #[test]
  fn geometry_for_common_f_over_d_values() {
    // f/D = 0.25：焦点恰在口径平面上，深度 = D/4 = f，张角 180°。
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.25)).unwrap();
    assert!(near(r.focal_length_m, 0.25, 1e-12) && near(r.depth_m, 0.25, 1e-12));
    assert!(near(r.focus_above_rim_m, 0.0, 1e-12));
    assert!(near(r.subtended_angle_deg, 180.0, 1e-9));

    // f/D = 0.4：f = 0.4 m，深 0.15625 m，焦点在口径平面前 0.24375 m，张角 128.0215°。
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.4)).unwrap();
    assert!(near(r.focal_length_m, 0.4, 1e-12) && near(r.depth_m, 0.156_25, 1e-12));
    assert!(near(r.focus_above_rim_m, 0.243_75, 1e-12));
    assert!(near(r.subtended_angle_deg, 128.0215, 1e-3));

    // f/D = 0.6：浅碗，张角约 90.48°。
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.6)).unwrap();
    assert!(near(r.depth_m, 0.104_167, 1e-6) && near(r.subtended_angle_deg, 90.4795, 1e-3));

    // 深碗（f/D < 0.25）：焦点落在碗里，位置为负，张角超过 180°。
    let r = analyze(&input(10_000.0, 1.0, 0.55, 0.2)).unwrap();
    assert!(r.focus_above_rim_m < 0.0, "{}", r.focus_above_rim_m);
    assert!(r.subtended_angle_deg > 180.0);
  }

  #[test]
  fn geometry_scales_with_diameter() {
    // 同一个 f/D：口径翻倍，焦距与深度都翻倍，张角不变。
    let a = analyze(&input(10_000.0, 1.0, 0.55, 0.4)).unwrap();
    let b = analyze(&input(10_000.0, 2.0, 0.55, 0.4)).unwrap();
    assert!(near(b.focal_length_m / a.focal_length_m, 2.0, 1e-12));
    assert!(near(b.depth_m / a.depth_m, 2.0, 1e-12));
    assert!(near(a.subtended_angle_deg, b.subtended_angle_deg, 1e-12));
  }

  #[test]
  fn focus_from_a_measured_depth() {
    // 口径 0.9 m、实测深 0.1 m：f = 0.81 / 1.6 = 0.50625 m，f/D = 0.5625。
    let (f, fd) = focus_from_depth(0.9, 0.1).unwrap();
    assert!(near(f, 0.506_25, 1e-12) && near(fd, 0.5625, 1e-12));
    // 往返：用得到的 f/D 重新算几何，深度应回到 0.1 m。
    let r = analyze(&input(10_000.0, 0.9, 0.55, fd)).unwrap();
    assert!(near(r.depth_m, 0.1, 1e-12) && near(r.focal_length_m, f, 1e-12));
    // 深度 = D/4 → f/D = 0.25。
    assert!(near(focus_from_depth(1.0, 0.25).unwrap().1, 0.25, 1e-12));
    for (d, depth) in [
      (0.0, 0.1),
      (0.9, 0.0),
      (-1.0, 0.1),
      (0.9, -0.1),
      (f64::NAN, 0.1),
      (0.9, f64::INFINITY),
    ] {
      assert_eq!(focus_from_depth(d, depth), None, "{d} {depth}");
    }
  }

  #[test]
  fn small_apertures_are_flagged() {
    // 145 MHz、1 m：λ = 2.07 m，口径只有 0.48 个波长 → 公式不可靠。
    assert!(
      analyze(&input(145.0, 1.0, 0.55, 0.4))
        .unwrap()
        .small_aperture
    );
    // 10 GHz、1 m：33 个波长。
    assert!(
      !analyze(&input(10_000.0, 1.0, 0.55, 0.4))
        .unwrap()
        .small_aperture
    );
    // 恰好 3 个波长不算「小」。
    let lambda = wavelength_m(1000.0).unwrap();
    assert!(
      !analyze(&input(1000.0, 3.0 * lambda, 0.55, 0.4))
        .unwrap()
        .small_aperture
    );
    assert!(
      analyze(&input(1000.0, 2.99 * lambda, 0.55, 0.4))
        .unwrap()
        .small_aperture
    );
  }

  #[test]
  fn rejects_nonsense() {
    let ok = input(10_000.0, 1.0, 0.55, 0.4);
    assert!(analyze(&ok).is_some());
    for bad in [
      DishInput {
        freq_mhz: 0.0,
        ..ok
      },
      DishInput {
        freq_mhz: -1.0,
        ..ok
      },
      DishInput {
        freq_mhz: f64::NAN,
        ..ok
      },
      DishInput {
        diameter_m: 0.0,
        ..ok
      },
      DishInput {
        diameter_m: -1.0,
        ..ok
      },
      DishInput {
        diameter_m: f64::INFINITY,
        ..ok
      },
      DishInput {
        efficiency: 0.0,
        ..ok
      },
      DishInput {
        efficiency: 1.01,
        ..ok
      },
      DishInput {
        efficiency: -0.5,
        ..ok
      },
      DishInput {
        efficiency: f64::NAN,
        ..ok
      },
      DishInput {
        f_over_d: 0.0,
        ..ok
      },
      DishInput {
        f_over_d: -0.4,
        ..ok
      },
      DishInput {
        f_over_d: f64::NAN,
        ..ok
      },
    ] {
      assert_eq!(analyze(&bad), None, "{bad:?}");
    }
    // 效率恰为 1 合法。
    assert!(
      analyze(&DishInput {
        efficiency: 1.0,
        ..ok
      })
      .is_some()
    );
  }
}
