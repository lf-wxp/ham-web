//! 陷波器（trap）：并联 LC 谐振电路，用于多波段偶极子。
//!
//! trap 并联谐振于高波段频率 `f_res`：在该频率呈现**高阻抗**（隔离外段），在更低频率
//! 呈现**感性**（加载外段）。于是「内段谐振于高波段、整副天线谐振于低波段」，一副天线
//! 通两个波段。
//!
//! 并联 LC 的电抗：`X = (X_L·X_C) / (X_C − X_L)`——谐振以下 `X_C > X_L` 为正（感性），
//! 谐振以上为负（容性），谐振处发散。
//!
//! # 外段长度的加载修正
//!
//! 低波段在 trap 谐振以下，trap 呈感性，像加载线圈一样把天线电气上拉长，外段要比
//! 「低波段半波 − 内段」更短。[`trap_dipole`] 用传输线模型的显式解计入这一项。
//! 不修正时，本站求解器里默认的 40 m + 20 m 设计低波段谐振比同长度直导线低约 6.6%；
//! 修正后只低约 0.5%。
//!
//! # 与 NEC 求解器的关系（自证与对拍）
//!
//! [`crate::nec::Load`] 是**串联** RLC（`X = X_L − X_C`，谐振处短路），表达不了并联 trap
//! 「谐振处开路」的行为，所以不能把 trap 当成一个频率无关的 `Load` 放进模型。
//! 但求解器**每次只算一个频率**：在该频率上，并联 LC 就是一个纯电抗 `X`（本模块的
//! [`trap_reactance_ohm`]），换成同一 `ω` 下电抗为 `X` 的串联电感（`X > 0`）或电容
//! （`X < 0`）后，求解器拿到的是同一个数。因此测试里按频点现算等效元件、逐点求解，
//! 不改 `Load`，也不新增 NEC 卡片类型。**这个等效只在单一频率上成立**，扫频要逐点重算。
//!
//! 测试（`mod tests`）用这个办法在自由空间里对一整根导线加两个 trap 求解，钉住：
//! 高 / 低两个波段各有且仅有一个串联谐振点；同样总长的直导线在 20 m 上没有谐振；
//! trap 谐振处外段几乎不通电流（隔离）、低波段外段参与辐射（加载）；修正把低波段谐振
//! 拉回同长度直导线的位置、高波段几乎不动。高波段这一项只能断言「变化小于 0.5%」：
//! trap 在高波段近似开路（`|X|` 上万欧）时，本站求解器 60 段的网格会把集总负载吸附到相邻
//! 节点，相当于内段长度被量化 ±½ 段长，谐振点有约 ±0.1% 的离散噪声（601 段的 necpp 没有）。
//!
//! 修正量本身另有一组对拍：固定在低波段频率上，在**长度域**求「直导线的谐振半长 −
//! 带 trap 的谐振半长」，两者在同一频率上求，求解器对 143/f 口径的系统偏差（自由空间里谐振
//! 偏长约 1.7%）互相抵消，剩下的就是 trap 的加载。对拍对象是 **necpp**（独立于本站求解器的
//! NEC-2 实现）：参照值是 `NECPP_CUT_M`，由 `crates/nec/reference/trap_necpp.py` 生成；
//! 本站求解器与 necpp 在这 9 组设计上相差 0.2%–3.6%，闭式缩短量与 necpp 的差别见 [`trap_dipole`]。
//!
//! 参照系说明：这是对**另一套 NEC-2 实现**的对拍，不是与实测的对拍，两套实现都是自由空间里
//! 的理想导线；求解器本身的精度边界见 `nec.rs` 模块文档（阻抗绝对值约有 10% 系统偏差，
//! 本模块用的谐振位置与长度差比阻抗绝对值可靠得多）。

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

/// 半波偶极的经验总长常数（m·MHz）：总长 `L = 143 / f`。
///
/// 比自由空间半波长 `149.9 / f` 短约 4.6%，折进了端效应与常见线径，是业余圈通用的取值。
/// 内段长度与「无 trap 时的外段长度」都以它为准。
const HALF_WAVE_M_MHZ: f64 = 143.0;

/// 估算偶极子平均特性阻抗用的导线半径（m）：2 mm 直径，约 12 AWG。
///
/// 它只进对数项（`ln(2h/a)`），影响不大，所以不做成输入：默认设计（40 m + 20 m、67 pF）上，
/// 常见线径（半径 0.5–2 mm）把外段在 +0.04 / −0.05 m 内挪动，折合谐振频率约 ±0.5%；
/// 极端的 0.25 mm 与 4 mm 也只是 +0.08 / −0.11 m。（公式算术，不依赖求解器。）
const WIRE_RADIUS_M: f64 = 1e-3;

/// 偶极子平均特性阻抗 `Z_a = 120·(ln(2h/a) − c)` 里的常数 `c`。
///
/// 教科书的 Schelkunoff 公式取 `c = 1`；这里取 **1.43**。依据：在 necpp 里对**裸偶极子**
/// （半径 0.5–4 mm、3.65–28.5 MHz，共 16 点）量谐振处的电抗斜率 `dX/dℓ`，反推出
/// `c ∈ [1.410, 1.475]`、均值 1.433，即 `c = 1` 的 `Z_a` 比谐振处的实际斜率高 4%–8%。
///
/// **没有用 trap 的数据拟合它。** 采用后，按闭式设计的 17 组 trap 偶极子（9 组设计集 + 8 组
/// 留出集，频段对与电容都不同）在 necpp 里的低波段谐振偏差**无一例外变小**：设计集最大
/// 1.38% → 0.83%、平均 0.67% → 0.44%；留出集最大 0.88% → 0.63%、平均 0.57% → 0.36%。
/// 若拿 trap 数据去拟合会得到更大的 `c`（`c = 2` 时设计集平均偏差只剩 0.09%），但那是用一个
/// 自由参数去贴同一个参照，不采用。复现：`crates/nec/reference/trap_necpp.py` 的 `slope` / `freq`。
const Z_C_OFFSET: f64 = 1.43;

/// 一副 trap 偶极子的设计结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrapDipole {
  /// trap 电感（µH）。
  pub l_uh: f64,
  /// trap 电容（pF）。
  pub c_pf: f64,
  /// 每臂内段长度（m，馈电点到 trap）。
  pub inner_m: f64,
  /// 每臂外段长度（m，trap 到末端）。已计入 trap 在低波段的感性加载造成的缩短，
  /// 所以比「低波段半波总长 − 内段总长」的一半更短（见 [`trap_dipole`]）。
  pub outer_m: f64,
}

/// 低波段上，把 trap 的感性加载计入后的每臂外段长度（m）。
///
/// 每臂看成一段**终端开路的传输线**：内段长 `p`、外段长 `q`，trap 的电抗 `X` 串在两段之间。
/// 从馈电点看进去，单臂阻抗为
///
/// ```text
/// Z_in = Z_c · (Z₂ + j·Z_c·tan θ_p) / (Z_c + j·Z₂·tan θ_p),    Z₂ = j·X − j·Z_c·cot θ_q
/// ```
///
/// 谐振即 `Z_in` 的电抗为零（分子为零），得到 `cot θ_q = tan θ_p + X / Z_c`，
/// 于是 `θ_q = atan2(1, tan θ_p + X / Z_c)` 有显式解，不用迭代。
///
/// - 电角度 `θ = 2π·f·长度 / (2·143)`：用「半波 = 143/f」的有效波长换算，所以 `X = 0` 时恰好退回
///   `q = 143/f/2 − p`（旧的闭式值），本函数只在其上扣掉 trap 的加载；
/// - `Z_c` 取偶极子平均特性阻抗 `Z_a = 120·(ln(2h/a) − c)` 的一半（`Z_a` 是整副偶极的，
///   单臂取一半），常数 `c` 见 [`Z_C_OFFSET`]，`h` 取无 trap 时的每臂长度，`a` 为 [`WIRE_RADIUS_M`]；
/// - `X` 是 trap 在**低波段**的电抗（[`trap_reactance_ohm`]），低波段在谐振以下所以 `X > 0`。
///
/// 返回 `None`：电抗算不出来（参数非法或 `X` 非有限）、`Z_c` 非正（只在导线比 2 mm 还短的
/// 极端高频才会出现）。
fn loaded_outer_m(l_uh: f64, c_pf: f64, f_low_mhz: f64, inner_m: f64) -> Option<f64> {
  let k = TAU * f_low_mhz / (2.0 * HALF_WAVE_M_MHZ);
  let arm_m = HALF_WAVE_M_MHZ / f_low_mhz / 2.0;
  let z_c = 0.5 * 120.0 * ((2.0 * arm_m / WIRE_RADIUS_M).ln() - Z_C_OFFSET);
  if !z_c.is_finite() || z_c <= 0.0 {
    return None;
  }
  let x = trap_reactance_ohm(l_uh, c_pf, f_low_mhz * 1e6)?;
  Some(1.0_f64.atan2((k * inner_m).tan() + x / z_c) / k)
}

/// 设计一副两波段 trap 偶极子。
///
/// `f_high_mhz` 为高波段（trap 谐振频率）、`f_low_mhz` 为低波段，`c_pf` 为选定的 trap 电容
/// （常用 50–100 pF）。内段取高波段半波偶极（每臂 143/f/2 米）。
///
/// # 外段：计入 trap 的感性加载
///
/// 低波段在 trap 谐振频率以下，trap 呈感性，等于在每臂中间串了一个加载线圈，把天线电气上
/// 拉长，所以外段要比「低波段半波总长 − 内段总长」的一半更短。缩短量由传输线模型的显式解
/// 给出（推导见内部函数 `loaded_outer_m`），不再需要用户自己凭经验剪线。
///
/// **精度**（对 necpp 的对拍，复现见 `crates/nec/reference/trap_necpp.py`）：
/// - **谐振频率**（这才是使用者关心的）：按本函数设计，17 组设计（电容 30–150 pF、低 / 高 =
///   0.26–0.95，其中 8 组是不参与取值的留出集）在 necpp 里的低波段谐振比目标低 0.04%–0.83%，
///   高波段在 −0.15%…+0.24% 以内。内段取「求解器里正好谐振于高波段的直导线长度」，
///   143/f 经验常数自身的偏差（自由空间里 1.7%）不计入；
/// - **缩短量本身**：低 / 高 = 0.26–0.51 的 6 组设计偏小 4%–10%（宁可欠修正，不会过修正）；
///   默认的 40 m + 20 m（67 pF）缩短 0.64 m，necpp 为 0.70 m。两个波段靠得近时相对偏差更大
///   （比值 0.71 / 0.85 / 0.95 为 18% / 30% / 56%），但缩短量本身只有 0.12–0.43 m，绝对差不到 1 dm。
///
/// **不需要修正的**（默认设计的一次性 necpp 测量；**脚本没有保留，这两组数字不能从仓库复现**，
/// 要依赖它们之前请先重测）：
/// - 内段：高波段谐振被 trap 自身的并联谐振部分「钉住」，对内段长度的敏感度不到低波段的一半
///   （内段 +4% → 高波段 −0.87%、低波段 −1.98%），按 143/f 取内段即可。
///   **别拿 143/f 的单独内段当基线**：求解器认为它偏短 1.7%、单独谐振在 14.43 MHz，
///   整副天线被 trap 拉到 14.25 MHz，那个「低 1.3%」是被拉动的量，不是外段的残余耦合；
/// - trap 线圈损耗：Q = 100 时谐振频率只移动 0.02%（低波段）/ 0.09%（高波段），不影响剪线长度；
///   但它抬高谐振电阻（低波段 +3 Ω，高波段 +29 Ω），影响驻波比与效率，这里不建模。
///
/// **没有建模的**：trap 的杂散电容（会把并联谐振点往下拉：67 pF 上多 5 pF 就是 −3.5%，
/// `c_pf` 请填含杂散电容的总值）、离地高度、导线绝缘层。精确长度仍以实测为准。
#[must_use]
pub fn trap_dipole(f_high_mhz: f64, f_low_mhz: f64, c_pf: f64) -> Option<TrapDipole> {
  // NaN 与任何数的比较都是 false，逐条 `<=` 拦不住它（`NaN <= 0.0` 也是 false），
  // 于是 NaN 会一路穿到结果里。先要求三个入参都是有限值。
  let finite = f_high_mhz.is_finite() && f_low_mhz.is_finite() && c_pf.is_finite();
  if !finite || f_high_mhz <= f_low_mhz || f_low_mhz <= 0.0 || c_pf <= 0.0 {
    return None;
  }
  let l_uh = trap_l_uh(f_high_mhz * 1e6, c_pf);
  let inner_m = HALF_WAVE_M_MHZ / f_high_mhz / 2.0;
  let outer_m = loaded_outer_m(l_uh, c_pf, f_low_mhz, inner_m)?;
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

  /// 不含 trap 加载修正的外段长度（旧的闭式值）：低波段半波总长减内段总长，再对半。
  fn uncorrected_outer_m(inner_m: f64, f_low_mhz: f64) -> f64 {
    HALF_WAVE_M_MHZ / f_low_mhz / 2.0 - inner_m
  }

  #[test]
  fn dipole_design_has_sane_dimensions() {
    // 40m + 20m trap 偶极：内段 ≈ 5.04 m；外段 143/7.05/2 − 5.04 = 5.10 m 扣掉 trap 的加载后 ≈ 4.46 m。
    let d = trap_dipole(14.175, 7.05, 67.0).unwrap();
    assert!((d.inner_m - 5.04).abs() < 0.05, "inner = {}", d.inner_m);
    assert!((d.outer_m - 4.46).abs() < 0.02, "outer = {}", d.outer_m);
    assert!((d.l_uh - 1.88).abs() < 0.05);
    let cut = uncorrected_outer_m(d.inner_m, 7.05) - d.outer_m;
    assert!((0.60..0.70).contains(&cut), "缩短量 {cut} m");
  }

  #[test]
  fn negligible_trap_leaves_the_closed_form_unchanged() {
    // 电容大到电感趋于 0（`X → 0`）：trap 等于一段导线，外段退回旧公式。
    // 这条钉死「X = 0 时修正项为零」，即 143/f 的口径没有被偷偷改动。
    let d = trap_dipole(14.175, 7.05, 1.0e9).expect("合法");
    let old = uncorrected_outer_m(d.inner_m, 7.05);
    assert!((d.outer_m - old).abs() < 1e-4, "{} vs {old}", d.outer_m);
  }

  #[test]
  fn a_heavier_trap_inductance_shortens_the_outer_arm_more() {
    // 同一对频率下，电容越小电感越大、低波段加载越重，外段越短。
    let outer = |c: f64| trap_dipole(14.175, 7.05, c).expect("合法").outer_m;
    let (small, mid, large) = (outer(30.0), outer(67.0), outer(150.0));
    assert!(small < mid && mid < large, "{small} {mid} {large}");
    assert!(large < uncorrected_outer_m(5.04, 7.05), "加载永远是缩短");
  }

  // ───────────────────── NEC 自证：加 trap 的天线多一个谐振点 ─────────────────────
  //
  // 模型：自由空间里沿 x 轴的一整根 1 mm 半径导线，馈电点在正中，两臂各 `inner + outer`，
  // trap 放在离中心 `±inner` 处。不用地面，是为了让判据只剩 trap 本身。

  use crate::nec::{Feed, Ground, Load, NecInput, NecResult, Wire, impedance, solve};

  /// 扫频类用例的分段数。60 与 100 段的谐振点相差不到 0.01 MHz，取小的省时间。
  const SEGMENTS: usize = 60;
  /// 导线半径（m）。
  const RADIUS_M: f64 = 1e-3;

  /// 把并联 trap 在 `freq_hz` 处的电抗，换成**该频点**的串联等效元件。
  ///
  /// `Load` 是串联 RLC，谐振处短路，表达不了「谐振处开路」的并联 trap。但求解器每次只算
  /// 一个频率，在这个频率上它只看见一个复阻抗数 `Z_L(ω)`；而并联 LC 在 `ω` 处是纯电抗
  /// `X`（`trap_reactance_ohm`，独立于 NEC 的闭式）。把它换成同一 `ω` 下电抗同为 `X` 的
  /// 串联电感（`X > 0`）或电容（`X < 0`），求解器拿到的就是同一个数。
  ///
  /// 所以这个等效**只对这一个频率成立**：扫频时每个频点都要重新换算，不能复用。
  fn trap_as_series_load(d: &TrapDipole, at: f64, freq_hz: f64) -> Load {
    let x = trap_reactance_ohm(d.l_uh, d.c_pf, freq_hz).expect("参数合法");
    let omega = TAU * freq_hz;
    let (l_uh, c_pf) = if x >= 0.0 {
      (x / omega * 1e6, 0.0)
    } else {
      (0.0, 1.0 / (omega * -x) * 1e12)
    };
    Load {
      wire: 0,
      at,
      r_ohm: 0.0,
      l_uh,
      c_pf,
    }
  }

  /// 一根沿 x 轴的单导线模型（馈电在正中）。`d = Some` 时在 `±inner` 处各放一个 trap。
  fn wire_input(
    freq_hz: f64,
    inner: f64,
    outer: f64,
    d: Option<&TrapDipole>,
    segments: usize,
  ) -> NecInput {
    let half = inner + outer;
    let offset = inner / (2.0 * half);
    NecInput {
      wires: vec![Wire::new(
        [-half, 0.0, 0.0],
        [half, 0.0, 0.0],
        RADIUS_M,
        segments,
      )],
      feeds: vec![Feed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: d.map_or_else(Vec::new, |d| {
        vec![
          trap_as_series_load(d, 0.5 - offset, freq_hz),
          trap_as_series_load(d, 0.5 + offset, freq_hz),
        ]
      }),
      freq_hz,
      ground: Ground::Free,
      conductivity: None,
    }
  }

  /// `traps = false` 时是同样总长的一根直导线；`true` 时在 `±inner` 处各放一个 trap。
  fn dipole_input(d: &TrapDipole, freq_hz: f64, traps: bool) -> NecInput {
    wire_input(freq_hz, d.inner_m, d.outer_m, traps.then_some(d), SEGMENTS)
  }

  /// 馈电点阻抗 `(R, X)`。
  fn feed_z(d: &TrapDipole, f_mhz: f64, traps: bool) -> (f64, f64) {
    impedance(&dipole_input(d, f_mhz * 1e6, traps)).expect("应能求解")
  }

  /// 割线法求 `g(x) = 0`。`g` 在根附近近似线性（馈电电抗对频率 / 长度正是如此）。
  fn secant_root(mut g: impl FnMut(f64) -> f64, x0: f64, x1: f64) -> f64 {
    let (mut a, mut b) = (x0, x1);
    let (mut ga, mut gb) = (g(a), g(b));
    for _ in 0..10 {
      if gb.abs() < 0.02 {
        return b;
      }
      let next = b - gb * (b - a) / (gb - ga);
      (a, ga) = (b, gb);
      b = next;
      gb = g(b);
    }
    assert!(gb.abs() < 0.5, "割线法没收敛：x = {b}, g = {gb}");
    b
  }

  /// 离 `f_guess_mhz` 最近的串联谐振点（MHz）。
  fn resonance_near(d: &TrapDipole, traps: bool, f_guess_mhz: f64) -> f64 {
    secant_root(|f| feed_z(d, f, traps).1, f_guess_mhz, f_guess_mhz * 1.01)
  }

  /// 在 `[lo, hi]` MHz 内找**串联谐振点**（电抗由负变正），返回 `(MHz, 此处电阻)`。
  ///
  /// 由正变负的是反谐振（电阻大到几千欧），不算。先按 0.1 MHz 扫，再对有变号的区间二分。
  fn series_resonances(d: &TrapDipole, traps: bool, lo_mhz: f64, hi_mhz: f64) -> Vec<(f64, f64)> {
    const STEP_MHZ: f64 = 0.1;
    let steps = ((hi_mhz - lo_mhz) / STEP_MHZ).round() as usize;
    let x_at = |f_mhz: f64| feed_z(d, f_mhz, traps).1;
    let mut found = Vec::new();
    let mut f0 = lo_mhz;
    let mut x0 = x_at(f0);
    for k in 1..=steps {
      let f1 = lo_mhz + STEP_MHZ * k as f64;
      let x1 = x_at(f1);
      if x0 < 0.0 && x1 >= 0.0 {
        let (mut a, mut b) = (f0, f1);
        for _ in 0..12 {
          let mid = 0.5 * (a + b);
          if x_at(mid) < 0.0 {
            a = mid;
          } else {
            b = mid;
          }
        }
        let f = 0.5 * (a + b);
        found.push((f, feed_z(d, f, traps).0));
      }
      (f0, x0) = (f1, x1);
    }
    found
  }

  /// 外段电流峰值 / 内段电流峰值（按各段中心的位置分内外）。
  fn outer_to_inner_current(d: &TrapDipole, r: &NecResult) -> f64 {
    let half = d.inner_m + d.outer_m;
    let peak = |outer: bool| {
      r.currents
        .iter()
        .enumerate()
        .filter(|&(i, _)| {
          let x = -half + (i as f64 + 0.5) * 2.0 * half / SEGMENTS as f64;
          (x.abs() > d.inner_m) == outer
        })
        .map(|(_, &c)| c)
        .fold(0.0_f64, f64::max)
    };
    peak(true) / peak(false)
  }

  #[test]
  fn series_equivalent_reproduces_the_parallel_lc_reactance() {
    // 整个自证的前提：求解器看到的阻抗，就是并联 LC 在该频率的电抗。两个符号都要覆盖
    // （谐振以下换成电感、以上换成电容），且不带电阻。
    let d = trap_dipole(14.175, 7.05, 67.0).expect("合法");
    for f_mhz in [3.6, 7.05, 10.0, 14.0, 14.3, 21.2, 28.5] {
      let f = f_mhz * 1e6;
      let want = trap_reactance_ohm(d.l_uh, d.c_pf, f).expect("合法");
      let got = trap_as_series_load(&d, 0.25, f).impedance(TAU * f);
      assert!(
        (got.im - want).abs() <= want.abs() * 1e-9,
        "{f_mhz} MHz：等效电抗 {} ≠ 并联 LC 的 {want}",
        got.im
      );
      assert_eq!(got.re, 0.0);
    }
  }

  #[test]
  fn trap_dipole_resonates_in_both_bands_in_nec() {
    // 40 m + 20 m 默认设计：高波段 14.175 MHz 为 trap 谐振点，低波段 7.05 MHz。
    let d = trap_dipole(14.175, 7.05, 67.0).expect("合法");
    let high = series_resonances(&d, true, 13.0, 15.5);
    let low = series_resonances(&d, true, 6.0, 7.6);
    assert_eq!(high.len(), 1, "高波段应恰有一个谐振点：{high:?}");
    assert_eq!(low.len(), 1, "低波段应恰有一个谐振点：{low:?}");
    let (f_high, r_high) = high[0];
    let (f_low, r_low) = low[0];

    // 高波段：trap 隔离外段，剩内段这副半波偶极；实测谐振比设计频率高约 0.45%。
    assert!(
      (f_high / 14.175 - 1.0).abs() < 0.015,
      "高波段谐振 {f_high} MHz 偏离设计 14.175 MHz 超过 1.5%"
    );
    // 自由空间半波偶极的谐振电阻在 70–80 Ω 量级（本求解器偏高约 10%，见 nec.rs 的精度表）。
    assert!((55.0..110.0).contains(&r_high), "高波段谐振电阻 {r_high} Ω");
    assert!((50.0..90.0).contains(&r_low), "低波段谐振电阻 {r_low} Ω");

    // 低波段：落在设计频率上方约 1.5%。求解器在自由空间里认为 143/f 的直导线谐振偏高 1.7%，
    // 这一项不是 trap 的事；没有加载修正时这里是 −5%（见 `correction_retunes_only_the_low_band_in_nec`）。
    let shift = f_low / 7.05 - 1.0;
    assert!(
      shift.abs() < 0.025,
      "低波段谐振 {f_low} MHz，相对设计 7.05 MHz 偏 {:.2}%",
      shift * 100.0
    );

    // 方向性的独立判据（不依赖具体数值）：同样总长的直导线谐振更高，
    // 因为它没有 trap 的感性加载。trap 的电抗符号写反会让这条翻过来。
    let plain = series_resonances(&d, false, 6.0, 8.0);
    assert_eq!(
      plain.len(),
      1,
      "无 trap 的直导线低波段应恰有一个谐振：{plain:?}"
    );
    assert!(
      f_low < plain[0].0 - 0.3,
      "加 trap 后的谐振 {f_low} MHz 应明显低于直导线的 {} MHz",
      plain[0].0
    );
  }

  #[test]
  fn the_same_wire_without_traps_has_no_resonance_on_the_high_band() {
    // 「多一个谐振点」的对照组：把 trap 换成一段导线，总长不变。20 m 频段上它约一个整波长，
    // 是电阻几千欧的反谐振，不存在低阻的串联谐振。
    let d = trap_dipole(14.175, 7.05, 67.0).expect("合法");
    assert_eq!(series_resonances(&d, false, 13.0, 15.5), vec![]);
    let (r, _) = feed_z(&d, 14.25, false);
    assert!(r > 2000.0, "20 m 上直导线的馈电电阻 {r} Ω 应是反谐振的高阻");
    // 同一频率、同一根导线，放上 trap 后是 80 Ω 左右的谐振：差的就是 trap。
    let (r_trap, x_trap) = feed_z(&d, 14.25, true);
    assert!(r_trap < 120.0 && x_trap.abs() < 30.0, "{r_trap} {x_trap}");
  }

  #[test]
  fn trap_isolates_the_outer_arm_at_resonance_and_loads_it_below() {
    // 直接看电流：trap 的两个作用（模块文档第一段）各有一个可量的现象。
    let d = trap_dipole(14.175, 7.05, 67.0).expect("合法");
    let ratio = |f_mhz: f64, traps: bool| {
      let r = solve(&dipole_input(&d, f_mhz * 1e6, traps)).expect("应能求解");
      outer_to_inner_current(&d, &r)
    };
    // 离 trap 谐振点 +0.1%（恰在谐振点上电抗发散，求解器会拒绝）：外段几乎不通电流。
    let f_trap = 14.175 * 1.001;
    let isolated = ratio(f_trap, true);
    assert!(
      isolated < 0.1,
      "trap 谐振处外段/内段电流比 {isolated} 应远小于 1"
    );
    // 对照：没有 trap 时外段与内段电流同量级。
    assert!(ratio(f_trap, false) > 0.9);
    // 低波段：trap 呈感性但不是开路，外段参与辐射。
    let loaded = ratio(7.05, true);
    assert!(loaded > 0.5, "低波段外段/内段电流比 {loaded} 应接近 1");
  }

  #[test]
  fn correction_retunes_only_the_low_band_in_nec() {
    // 参照物：同一名义总长（143/f）的**直导线**在同一求解器里的谐振。求解器对 143/f 有自己的
    // 口径偏差（自由空间里偏高约 1.7%），拿它当参照，这一项就和 trap 无关了。
    let d = trap_dipole(14.175, 7.05, 67.0).expect("合法");
    let raw = TrapDipole {
      outer_m: uncorrected_outer_m(d.inner_m, 7.05),
      ..d
    };
    let plain = resonance_near(&raw, false, 7.1);
    let before = resonance_near(&raw, true, 6.8);
    let after = resonance_near(&d, true, 7.1);
    let (shift_before, shift_after) = (before / plain - 1.0, after / plain - 1.0);

    // 不修正：trap 的感性加载把谐振压低五个百分点以上（实测 6.6%），
    // 整个 40 m 业余段（2.8% 宽）都落在外面。
    assert!(
      shift_before < -0.05,
      "未修正的低波段谐振 {before} MHz，相对直导线 {plain} MHz 只偏 {:.2}%",
      shift_before * 100.0
    );
    // 修正后：回到直导线的位置附近，残差不到 1%（实测 0.5%）。
    assert!(
      shift_after.abs() < 0.01,
      "修正后低波段谐振 {after} MHz，相对直导线 {plain} MHz 偏 {:.2}%",
      shift_after * 100.0
    );

    // 高波段由 trap 隔离，外段缩短 0.6 m 几乎不动它（实测变化 < 0.1%，在求解器 60 段的
    // 离散噪声量级内；见模块文档，所以只断言不超过 0.5%）。
    let high_before = resonance_near(&raw, true, 14.2);
    let high_after = resonance_near(&d, true, 14.2);
    assert!(
      (high_after / high_before - 1.0).abs() < 0.005,
      "高波段谐振从 {high_before} 变到 {high_after} MHz，不该动这么多"
    );
  }

  /// 低波段长度域求根用的分段数：40 段与 80 段给出的缩短量相差不到 0.01 m。
  /// 高波段（trap 附近的电流变化快）不能这么稀，扫频类用例仍用 [`SEGMENTS`]。
  const LOW_BAND_SEGMENTS: usize = 40;

  /// 固定在低波段频率上的馈电电抗；`d = None` 且 `inner = 0` 即直导线。
  fn low_band_reactance(f_low_mhz: f64, inner: f64, outer: f64, d: Option<&TrapDipole>) -> f64 {
    let input = wire_input(f_low_mhz * 1e6, inner, outer, d, LOW_BAND_SEGMENTS);
    impedance(&input).expect("应能求解").1
  }

  /// NEC 给出的缩短量（每臂，m）：直导线的谐振半长 − 带 trap 的谐振半长（内段不变、调外段）。
  ///
  /// 两者都固定在**同一个低波段频率**上、在长度域求谐振：求解器对 143/f 口径的偏差
  /// （直导线谐振半长比 143/f/2 长约 1.7%）在两边相同，相减后抵消。
  fn nec_cut_m(f_high_mhz: f64, f_low_mhz: f64, c_pf: f64) -> f64 {
    let d = trap_dipole(f_high_mhz, f_low_mhz, c_pf).expect("合法");
    let arm = HALF_WAVE_M_MHZ / f_low_mhz / 2.0;
    let plain = secant_root(
      |h| low_band_reactance(f_low_mhz, 0.0, h, None),
      arm,
      arm * 1.01,
    );
    let outer = secant_root(
      |q| low_band_reactance(f_low_mhz, d.inner_m, q, Some(&d)),
      d.outer_m,
      d.outer_m * 0.9,
    );
    plain - (d.inner_m + outer)
  }

  /// necpp（PyNEC 2.4.0）算出的每臂缩短量（m）：`(高波段 MHz, 低波段 MHz, 电容 pF, 缩短量)`。
  ///
  /// 与 [`nec_cut_m`] 同一口径（自由空间、1 mm 半径、内段取 143/f/2、直导线谐振半长 −
  /// 内段 − 带 trap 的谐振外段）。necpp 是独立于本站求解器的 NEC-2 实现；
  /// 由 `crates/nec/reference/trap_necpp.py cut` 生成，改口径要重跑它再抄回来。
  /// 前 6 组低 / 高 = 0.26–0.51（典型设计），后 3 组两个波段靠得近（0.71 / 0.85 / 0.95）。
  const NECPP_CUT_M: [(f64, f64, f64, f64); 9] = [
    (14.175, 7.05, 67.0, 0.700),  // 40 m + 20 m 默认
    (14.175, 7.05, 30.0, 1.395),  // 小电容 → 大电感，加载重
    (14.175, 7.05, 150.0, 0.330), // 大电容 → 小电感，加载轻
    (14.175, 3.65, 67.0, 0.895),  // 80 m + 20 m
    (28.5, 14.175, 67.0, 0.196),  // 20 m + 10 m
    (7.1, 3.65, 100.0, 1.634),    // 80 m + 40 m
    (14.175, 10.12, 67.0, 0.434), // 近
    (14.175, 12.0, 67.0, 0.261),  // 更近
    (14.175, 13.5, 67.0, 0.123),  // 最近
  ];

  #[test]
  fn solver_cut_agrees_with_necpp() {
    // 本站求解器与 necpp 是两套独立实现：这条守住「本模块用的 NEC 参照没有漂移」。
    // 实测相差 0.2%–3.6%（最大在最近的一组，缩短量只有 0.12 m）。
    for (f_high, f_low, c_pf, necpp) in NECPP_CUT_M {
      let ours = nec_cut_m(f_high, f_low, c_pf);
      assert!(
        (ours - necpp).abs() <= 0.05 * necpp,
        "{f_high}/{f_low} MHz、{c_pf} pF：本站求解器 {ours:.4} m，necpp {necpp:.3} m"
      );
    }
  }

  #[test]
  fn closed_form_cut_tracks_necpp() {
    for (f_high, f_low, c_pf, necpp) in NECPP_CUT_M {
      let d = trap_dipole(f_high, f_low, c_pf).expect("合法");
      let closed = uncorrected_outer_m(d.inner_m, f_low) - d.outer_m;
      if f_low / f_high < 0.6 {
        // 典型设计：缩短量比 necpp 小 4%–10%，宁可欠修正、不会过修正。下界 0.89 是为了抓住
        // 「`Z_C_OFFSET` 退回教科书的 1」（那时是 0.866–0.919，会掉到下界外）。
        let ratio = closed / necpp;
        assert!(
          (0.89..1.0).contains(&ratio),
          "{f_high}/{f_low} MHz、{c_pf} pF：闭式缩短 {closed:.3} m，necpp {necpp:.3} m，比值 {ratio:.3}"
        );
      } else {
        // 两个波段靠得近：闭式偏小得更多（相对偏差 18%–56%），但此时 trap 电抗很大、
        // 缩短量本身只有 0.12–0.43 m，所以看绝对差。
        assert!(
          closed > 0.0 && (closed - necpp).abs() < 0.1,
          "{f_high}/{f_low} MHz：闭式缩短 {closed:.3} m，necpp {necpp:.3} m"
        );
      }
    }
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
    // 低波段高到偶极子比 2 mm 导线还短（50 THz）：平均特性阻抗算出负数，直接说算不出来，
    // 而不是把负的 `Z_c` 带进 `atan2` 得到一个看起来像长度的数。
    assert_eq!(trap_dipole(1.0e8, 5.0e7, 67.0), None);
  }
}
