//! 电短天线加载线圈：短单极子（车载鞭天线）谐振所需的加载电感。
//!
//! 车载 / 移动 HF 天线几乎都是「物理上远短于 λ/4」的鞭状单极子，呈强容性，
//! 要在根部串一段加载线圈把容抗抵消、让天线谐振。这里按**短单极子（h ≤ λ/4）**
//! 的闭式近似估算（越短越准，逼近 λ/4 后精度下降）：
//!
//! - 辐射电阻 `Rr = 40π²·(h/λ)²`；
//! - 天线电容 `C = 2πε₀·h / [ln(2h/a) − 1]`（`a` 是导体半径，镜像法）；
//! - 容抗 `Xc = 1/(ωC)`，加载电感 `L = Xc/ω = 1/(ω²C)`（根部加载）。
//!
//! `L·C` 谐振回 `f` 是定义自洽的；真正的物理量是电容的闭式近似，只适用于
//! `h ≤ λ/4`（越短越准），且未建模地面损耗、线圈自身损耗与分布电容 —— 结果是**估算**，
//! 精确谐振点以实测为准。验证：Rr 随 h² 缩放、电容随 h 单调增、算例量级合理。

use crate::waveguide::C;

/// 真空介电常数（F/m）。
const EPSILON0: f64 = 8.854_187_812_8e-12;

/// 短单极子参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadingCoilInput {
  /// 天线高度（m）。
  pub height_m: f64,
  /// 导体直径（mm）。
  pub conductor_mm: f64,
  /// 工作频率（MHz）。
  pub freq_mhz: f64,
}

/// 加载线圈估算结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadingCoil {
  /// 辐射电阻（Ω）。
  pub radiation_resistance: f64,
  /// 天线电容（F）。
  pub capacitance_f: f64,
  /// 容抗（Ω，正值表示容性）。
  pub capacitive_reactance: f64,
  /// 所需加载电感（H）。
  pub inductance_h: f64,
  /// 是否仍在「电短单极子（h ≤ λ/4）」的适用域内。
  ///
  /// `false` 时上面的数值只能当数量级参考（闭式近似在 h 逼近 λ/4 后迅速失真），
  /// 界面必须据此给出提示 —— 不能只在模块文档里写一句「估算」就算了。
  pub electrically_short: bool,
}

/// 估算短单极子的加载线圈；输入不合法返回 `None`。
#[must_use]
pub fn loading_coil(input: LoadingCoilInput) -> Option<LoadingCoil> {
  let LoadingCoilInput {
    height_m,
    conductor_mm,
    freq_mhz,
  } = input;
  if height_m <= 0.0
    || conductor_mm <= 0.0
    || freq_mhz <= 0.0
    || !height_m.is_finite()
    || !conductor_mm.is_finite()
    || !freq_mhz.is_finite()
  {
    return None;
  }
  let f = freq_mhz * 1e6;
  let lambda = C / f;
  // 只在「电短」范围成立：超过 λ/4 就不是「加载」而是「缩短」了，数值仍算（用户要看得见量级），
  // 但用这个标志把「已出适用域」交给界面提示。
  let electrically_short = height_m <= lambda / 4.0;
  let radiation_resistance = 40.0 * std::f64::consts::PI.powi(2) * (height_m / lambda).powi(2);
  let a = conductor_mm / 1000.0 / 2.0;
  // ln(2h/a) − 1 > 0 才有正电容：要求 2h/a > e（≈ 2.72）。
  if 2.0 * height_m / a <= std::f64::consts::E {
    return None;
  }
  let capacitance_f =
    2.0 * std::f64::consts::PI * EPSILON0 * height_m / ((2.0 * height_m / a).ln() - 1.0);
  let omega = std::f64::consts::TAU * f;
  let capacitive_reactance = 1.0 / (omega * capacitance_f);
  let inductance_h = 1.0 / (omega * omega * capacitance_f);
  Some(LoadingCoil {
    radiation_resistance,
    capacitance_f,
    capacitive_reactance,
    inductance_h,
    electrically_short,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loading_coil_resonates_at_frequency() {
    // L·C 应满足 1/(2π√(LC)) = f（往返自洽）。
    for &(h, d, f) in &[(2.4, 2.0, 7.0), (1.0, 3.0, 3.8), (0.8, 1.0, 14.2)] {
      let r = loading_coil(LoadingCoilInput {
        height_m: h,
        conductor_mm: d,
        freq_mhz: f,
      })
      .unwrap();
      let resonant = 1.0 / (std::f64::consts::TAU * (r.inductance_h * r.capacitance_f).sqrt());
      assert!((resonant - f * 1e6).abs() / (f * 1e6) < 1e-9);
    }
  }

  #[test]
  fn radiation_resistance_scales_with_height_squared() {
    let a = loading_coil(LoadingCoilInput {
      height_m: 2.4,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    let b = loading_coil(LoadingCoilInput {
      height_m: 4.8,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    // 高度翻倍 → Rr × 4。
    assert!((b.radiation_resistance / a.radiation_resistance - 4.0).abs() < 1e-9);
  }

  #[test]
  fn capacitance_grows_with_height() {
    let a = loading_coil(LoadingCoilInput {
      height_m: 1.0,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    let b = loading_coil(LoadingCoilInput {
      height_m: 2.0,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    assert!(b.capacitance_f > a.capacitance_f);
    // 2.4 m 鞭 @ 7 MHz 的加载电感落在几十 µH 量级（车载 40 m 天线的经验范围）。
    let r = loading_coil(LoadingCoilInput {
      height_m: 2.4,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    let uh = r.inductance_h * 1e6;
    assert!((15.0..60.0).contains(&uh), "L = {uh} µH");
  }

  #[test]
  fn invalid_inputs_return_none() {
    let p = LoadingCoilInput {
      height_m: 2.4,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    };
    assert_eq!(loading_coil(LoadingCoilInput { height_m: 0.0, ..p }), None);
    assert_eq!(
      loading_coil(LoadingCoilInput {
        conductor_mm: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(loading_coil(LoadingCoilInput { freq_mhz: 0.0, ..p }), None);
    // 太「粗」：2h/a ≤ e（h/a ≤ 1.36）时电容公式失效。
    assert_eq!(
      loading_coil(LoadingCoilInput {
        height_m: 0.01,
        conductor_mm: 20.0,
        ..p
      }),
      None
    );
  }

  #[test]
  fn applicability_flag_tracks_the_quarter_wavelength_limit() {
    // 2.4 m 鞭 @ 7 MHz：λ/4 ≈ 10.7 m，仍在电短范围。
    let ok = loading_coil(LoadingCoilInput {
      height_m: 2.4,
      conductor_mm: 2.0,
      freq_mhz: 7.0,
    })
    .unwrap();
    assert!(ok.electrically_short);
    // 10 m 鞭 @ 28 MHz：λ/4 ≈ 2.7 m，早已不是「电短」，标志必须为 false 让界面提示。
    let out = loading_coil(LoadingCoilInput {
      height_m: 10.0,
      conductor_mm: 2.0,
      freq_mhz: 28.0,
    })
    .unwrap();
    assert!(!out.electrically_short);
  }
}
