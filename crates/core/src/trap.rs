//! 陷波器（trap）：并联 LC 谐振电路，用于多波段偶极子。
//!
//! trap 并联谐振于高波段频率 `f_res`：在该频率呈现**高阻抗**（隔离外段），在更低频率
//! 呈现**感性**（加载外段）。于是「内段谐振于高波段、整副天线谐振于低波段」，一副天线
//! 通两个波段。
//!
//! 并联 LC 的电抗：`X = (X_L·X_C) / (X_C − X_L)`——谐振以下 `X_C > X_L` 为正（感性），
//! 谐振以上为负（容性），谐振处发散。
//!
//! > 注意：站内 NEC 求解器的 [`crate::nec::Load`] 建模的是**串联** RLC（`X = X_L − X_C`，
//! > 谐振处短路），与并联 trap 的「谐振处开路」行为相反，无法直接表达并联 trap。所以
//! > trap 的谐振 / 加载计算在这里；「加 trap 的 NEC 模型比无 trap 多一个谐振点」的自证
//! > 留待 `Load` 支持并联 LC 后补。

use std::f64::consts::TAU;

/// 由并联谐振频率 `f_res_hz` 与电容 `c_pf` 求电感（µH）。
///
/// 前置条件：两者都是正的有限值（否则结果是 inf / NaN）。带校验的入口是
/// [`trap_dipole`]，页面一律走它。
#[must_use]
pub fn trap_l_uh(f_res_hz: f64, c_pf: f64) -> f64 {
  debug_assert!(
    f_res_hz.is_finite() && f_res_hz > 0.0 && c_pf.is_finite() && c_pf > 0.0,
    "trap_l_uh 需要正的有限参数：f={f_res_hz}, c={c_pf}"
  );
  1e6 / ((TAU * f_res_hz).powi(2) * c_pf * 1e-12)
}

/// 由并联谐振频率 `f_res_hz` 与电感 `l_uh` 求电容（pF）。前置条件同 [`trap_l_uh`]。
#[must_use]
pub fn trap_c_pf(f_res_hz: f64, l_uh: f64) -> f64 {
  debug_assert!(
    f_res_hz.is_finite() && f_res_hz > 0.0 && l_uh.is_finite() && l_uh > 0.0,
    "trap_c_pf 需要正的有限参数：f={f_res_hz}, l={l_uh}"
  );
  1e12 / ((TAU * f_res_hz).powi(2) * l_uh * 1e-6)
}

/// 并联 LC 的谐振频率（Hz）。前置条件同 [`trap_l_uh`]。
#[must_use]
pub fn trap_resonance_hz(l_uh: f64, c_pf: f64) -> f64 {
  debug_assert!(
    l_uh.is_finite() && l_uh > 0.0 && c_pf.is_finite() && c_pf > 0.0,
    "trap_resonance_hz 需要正的有限参数：l={l_uh}, c={c_pf}"
  );
  1.0 / (TAU * (l_uh * 1e-6 * c_pf * 1e-12).sqrt())
}

/// 并联 LC 在 `freq_hz` 处的电抗（Ω）。
///
/// 谐振以下为正（感性，加载）、以上为负（容性）；`freq_hz` 恰在谐振点时发散（`±∞`）。
/// 参数非法（频率 / 电感 / 电容不是正的有限值）时返回 `None`：`freq_hz = 0` 会让
/// `(0 × inf) / (inf − 0)` 得到 NaN，静默画出一条空曲线。
#[must_use]
pub fn trap_reactance_ohm(l_uh: f64, c_pf: f64, freq_hz: f64) -> Option<f64> {
  if !freq_hz.is_finite() || freq_hz <= 0.0 || !l_uh.is_finite() || l_uh <= 0.0 {
    return None;
  }
  if !c_pf.is_finite() || c_pf <= 0.0 {
    return None;
  }
  let xl = TAU * freq_hz * l_uh * 1e-6;
  let xc = 1.0 / (TAU * freq_hz * c_pf * 1e-12);
  Some((xl * xc) / (xc - xl))
}

/// 一副 trap 偶极子的设计结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrapDipole {
  /// trap 电感（µH）。
  pub l_uh: f64,
  /// trap 电容（pF）。
  pub c_pf: f64,
  /// 每臂内段长度（m，馈电点到 trap）。
  pub inner_m: f64,
  /// 每臂外段长度（m，trap 到末端）。
  pub outer_m: f64,
}

/// 设计一副两波段 trap 偶极子。
///
/// `f_high_mhz` 为高波段（trap 谐振频率）、`f_low_mhz` 为低波段，`c_pf` 为选定的 trap 电容
/// （常用 50–100 pF）。内段取高波段半波偶极（每臂 143/f/2 米），外段取「低波段半波总长 −
/// 内段总长」的一半；trap 在低波段的感性加载会再缩短外段，属闭式近似，精确长度以实测为准。
#[must_use]
pub fn trap_dipole(f_high_mhz: f64, f_low_mhz: f64, c_pf: f64) -> Option<TrapDipole> {
  // NaN 与任何数的比较都是 false，逐条 `<=` 拦不住它（`NaN <= 0.0` 也是 false），
  // 于是 NaN 会一路穿到结果里。先要求三个入参都是有限值。
  let finite = f_high_mhz.is_finite() && f_low_mhz.is_finite() && c_pf.is_finite();
  if !finite || f_high_mhz <= f_low_mhz || f_low_mhz <= 0.0 || c_pf <= 0.0 {
    return None;
  }
  let l_uh = trap_l_uh(f_high_mhz * 1e6, c_pf);
  let inner_m = 143.0 / f_high_mhz / 2.0;
  let total_low_m = 143.0 / f_low_mhz;
  let outer_m = (total_low_m - inner_m * 2.0) / 2.0;
  // `outer_m <= 0.0` 同样放行 NaN，所以先要求它有限再看符号。
  if !outer_m.is_finite() || outer_m <= 0.0 || !l_uh.is_finite() {
    return None;
  }
  Some(TrapDipole {
    l_uh,
    c_pf,
    inner_m,
    outer_m,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn l_and_c_round_trip_through_resonance() {
    let f = 14.175e6;
    let l = trap_l_uh(f, 67.0);
    // 67 pF 在 14.175 MHz 并联谐振的电感 ≈ 1.88 µH。
    assert!((l - 1.88).abs() < 0.02, "L = {l}");
    assert!((trap_resonance_hz(l, 67.0) - f).abs() / f < 1e-9);
    assert!((trap_c_pf(f, l) - 67.0).abs() < 1e-6);
  }

  #[test]
  fn reactance_is_inductive_below_and_capacitive_above() {
    let f = 14.175e6;
    let l = trap_l_uh(f, 67.0);
    // 谐振以下（如 40m = 7.05 MHz）感性、以上（如 15m = 21.2 MHz）容性。
    assert!(trap_reactance_ohm(l, 67.0, 7.05e6).expect("低频") > 0.0);
    assert!(trap_reactance_ohm(l, 67.0, 21.2e6).expect("高频") < 0.0);
    // 0 Hz / 非有限参数不再返回 NaN：直接说「算不出来」。
    assert_eq!(trap_reactance_ohm(l, 67.0, 0.0), None);
    assert_eq!(trap_reactance_ohm(l, 67.0, -1.0), None);
    assert_eq!(trap_reactance_ohm(l, 67.0, f64::NAN), None);
    assert_eq!(trap_reactance_ohm(l, 0.0, 14.0e6), None);
  }

  #[test]
  fn dipole_design_has_sane_dimensions() {
    // 40m + 20m trap 偶极：内段 ≈ 5.0 m、外段 ≈ 5.1 m。
    let d = trap_dipole(14.175, 7.05, 67.0).unwrap();
    assert!((d.inner_m - 5.04).abs() < 0.05, "inner = {}", d.inner_m);
    assert!((d.outer_m - 5.10).abs() < 0.05, "outer = {}", d.outer_m);
    assert!((d.l_uh - 1.88).abs() < 0.05);
  }

  #[test]
  fn invalid_designs_return_none() {
    assert_eq!(trap_dipole(7.0, 14.0, 67.0), None); // 高低波段反了
    assert_eq!(trap_dipole(14.0, 14.0, 67.0), None); // 相等
    assert_eq!(trap_dipole(14.0, 7.0, 0.0), None); // 电容非正
    assert_eq!(trap_dipole(0.0, 7.0, 67.0), None);
    // NaN 与任何数比较都是 false，必须显式挡掉，不能让它漏进结果。
    assert_eq!(trap_dipole(f64::NAN, 7.0, 67.0), None);
    assert_eq!(trap_dipole(14.0, f64::NAN, 67.0), None);
    assert_eq!(trap_dipole(14.0, 7.0, f64::NAN), None);
    assert_eq!(trap_dipole(f64::INFINITY, 7.0, 67.0), None);
  }
}
