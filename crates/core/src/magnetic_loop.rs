//! 磁环小环天线（magnetic loop）：圆环天线的电感、辐射 / 损耗电阻、效率、Q、带宽、
//! 调谐电容与电容端电压。
//!
//! 小环天线（环周长 ≪ λ）是阳台 / 受限场地的常用方案，代价是辐射电阻极小、Q 极高、
//! 调谐电容两端电压到几千伏。这里用闭式公式估算，全部是公开的经典口径：
//!
//! - 电感 `L = μ0·N²·(D/2)·[ln(8D/d) − 2]`（圆环，D 环径、d 导体直径）；
//! - 辐射电阻 `Rr = 320π⁴·(N·A/λ²)²`（A 环面积）；
//! - 损耗电阻 `Rl = (D·N/d)·√(π·f·μ0·ρ)`（铜导体趋肤效应）；
//! - 效率 η = Rr/(Rr+Rl)，Q = ωL/(Rr+Rl)，带宽 = f/Q；
//! - 谐振调谐电容 `C = 1/(ω²L)`，电容端电压 `V = Q·√(P·(Rr+Rl))`。
//!
//! 验证：`L` 与 `C` 往返谐振回 `f`（`1/(2π√(LC)) = f`），并钉死 1 m 环的电感算例。

use crate::waveguide::C;

/// 真空磁导率（H/m）。
const MU0: f64 = 4.0e-7 * std::f64::consts::PI;
/// 铜电阻率（Ω·m）。
const COPPER_RHO: f64 = 1.68e-8;

/// 磁环天线参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopParams {
  /// 环直径（m）。
  pub diameter_m: f64,
  /// 导体直径（mm）。
  pub conductor_mm: f64,
  /// 工作频率（MHz）。
  pub freq_mhz: f64,
  /// 匝数（正整数，通常 1）。
  ///
  /// 用 `f64` 只是为了和其余参数同一套输入口径；半匝没有物理意义，非整数按非法参数拒绝。
  pub turns: f64,
  /// 发射功率（W）。
  pub power_w: f64,
}

/// 磁环天线的估算结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagneticLoop {
  /// 电感（H）。
  pub inductance_h: f64,
  /// 辐射电阻（Ω）。
  pub radiation_resistance: f64,
  /// 损耗电阻（Ω）。
  pub loss_resistance: f64,
  /// 效率（0..1）。
  pub efficiency: f64,
  /// 品质因数。
  pub q: f64,
  /// 带宽（Hz）。
  pub bandwidth_hz: f64,
  /// 谐振调谐电容（F）。
  pub tuning_capacitance_f: f64,
  /// 电容两端电压（V）。
  pub cap_voltage: f64,
  /// 是否仍在「电小环（周长 ≤ λ/2π）」的适用域内。
  ///
  /// `false` 时上面的数值只能当数量级参考：小环近似（均匀电流、辐射电阻 ∝ A²/λ⁴）在周长
  /// 接近 λ 后迅速失真，界面必须据此提示。用 λ/2π 而不是更保守的 λ/10，是为了让常见的
  /// 1 m 环在 14 MHz（周长 3.14 m、λ/2π ≈ 3.36 m）不被误报。
  pub electrically_small: bool,
}

/// 估算磁环小环天线；输入不合法（非正尺寸 / 频率 / 匝数、负功率、环太「粗」）返回 `None`。
///
/// 匝数必须是正整数：`1.5` 匝没有物理意义，公式却会照算出「半匝环」的电感，用户读到的是一份
/// 不存在的天线参数。
#[must_use]
pub fn magnetic_loop(p: LoopParams) -> Option<MagneticLoop> {
  if p.diameter_m <= 0.0
    || p.conductor_mm <= 0.0
    || p.freq_mhz <= 0.0
    || p.turns < 1.0
    || (p.turns - p.turns.round()).abs() > 1e-9
    || p.power_w < 0.0
    || !p.diameter_m.is_finite()
    || !p.conductor_mm.is_finite()
    || !p.freq_mhz.is_finite()
    || !p.turns.is_finite()
    || !p.power_w.is_finite()
  {
    return None;
  }
  let d = p.conductor_mm / 1000.0;
  // 8D/d 同时是电感公式的对数自变量与「环是否太粗」的判据，只算一次。
  let ratio = 8.0 * p.diameter_m / d;
  // ln(8D/d) − 2 > 0 才有正电感：要求 8D/d > e²（≈ 7.39）。
  if ratio <= std::f64::consts::E.powi(2) {
    return None;
  }
  let f = p.freq_mhz * 1e6;
  let omega = std::f64::consts::TAU * f;
  let area = std::f64::consts::PI * p.diameter_m.powi(2) / 4.0;
  let lambda = C / f;

  // L = μ0·N²·(D/2)·[ln(8D/d) − 2]。
  let inductance_h = MU0 * p.turns.powi(2) * (p.diameter_m / 2.0) * (ratio.ln() - 2.0);
  let radiation_resistance =
    320.0 * std::f64::consts::PI.powi(4) * (p.turns * area / lambda.powi(2)).powi(2);
  let loss_resistance =
    (p.diameter_m * p.turns / d) * (std::f64::consts::PI * f * MU0 * COPPER_RHO).sqrt();
  let series = radiation_resistance + loss_resistance;
  let q = omega * inductance_h / series;
  let efficiency = radiation_resistance / series;
  let bandwidth_hz = f / q;
  let tuning_capacitance_f = 1.0 / (omega * omega * inductance_h);
  let cap_voltage = q * (p.power_w * series).sqrt();
  // 环的周长（每匝一圈，匝数不改变环的尺寸）。
  let perimeter = std::f64::consts::PI * p.diameter_m;
  let electrically_small = perimeter <= lambda / std::f64::consts::TAU;

  Some(MagneticLoop {
    inductance_h,
    radiation_resistance,
    loss_resistance,
    efficiency,
    q,
    bandwidth_hz,
    tuning_capacitance_f,
    cap_voltage,
    electrically_small,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn loop_at(diameter_m: f64, conductor_mm: f64, freq_mhz: f64, turns: f64) -> MagneticLoop {
    magnetic_loop(LoopParams {
      diameter_m,
      conductor_mm,
      freq_mhz,
      turns,
      power_w: 100.0,
    })
    .expect("参数应合法")
  }

  #[test]
  fn inductance_known_example() {
    // 1 m 环、10 mm 铜管：L = μ0·0.5·[ln(800) − 2] ≈ 2.94 µH。
    let l = loop_at(1.0, 10.0, 14.0, 1.0);
    assert!(
      (l.inductance_h - 2.94e-6).abs() < 0.05e-6,
      "L = {} H",
      l.inductance_h
    );
  }

  #[test]
  fn tuning_capacitance_resonates_with_inductance() {
    // 往返：L 与 C 应满足 1/(2π√(LC)) = f。
    for &(d, c, f, n) in &[
      (1.0, 10.0, 14.0, 1.0),
      (0.5, 6.0, 7.1, 1.0),
      (1.2, 15.0, 3.8, 2.0),
    ] {
      let l = loop_at(d, c, f, n);
      let resonant =
        1.0 / (std::f64::consts::TAU * (l.inductance_h * l.tuning_capacitance_f).sqrt());
      assert!(
        (resonant - f * 1e6).abs() / (f * 1e6) < 1e-9,
        "谐振 {} vs {} MHz",
        resonant,
        f
      );
    }
  }

  #[test]
  fn efficiency_is_within_bounds() {
    for &(d, c, f, n) in &[
      (1.0, 10.0, 14.0, 1.0),
      (2.0, 20.0, 7.0, 1.0),
      (0.8, 5.0, 21.0, 1.0),
    ] {
      let l = loop_at(d, c, f, n);
      assert!((0.0..=1.0).contains(&l.efficiency), "效率 {}", l.efficiency);
      assert!(l.q > 0.0 && l.bandwidth_hz > 0.0 && l.cap_voltage > 0.0);
    }
  }

  #[test]
  fn small_loop_has_tiny_radiation_resistance() {
    // 1 m 环在 14 MHz 的辐射电阻 ~0.09 Ω，远小于损耗电阻，效率 < 100%。
    let l = loop_at(1.0, 10.0, 14.0, 1.0);
    assert!(
      l.radiation_resistance < 0.2,
      "Rr = {}",
      l.radiation_resistance
    );
    assert!(l.radiation_resistance < l.loss_resistance);
  }

  #[test]
  fn applicability_flag_tracks_the_small_loop_limit() {
    // 1 m 环 @ 14 MHz：周长 3.14 m < λ/2π ≈ 3.36 m，仍是「电小」。
    assert!(loop_at(1.0, 10.0, 14.0, 1.0).electrically_small);
    // 10 m 环 @ 144 MHz：周长 31.4 m ≫ λ/2π ≈ 0.33 m，小环近似已失效，界面必须提示。
    assert!(!loop_at(10.0, 10.0, 144.0, 1.0).electrically_small);
  }

  #[test]
  fn cap_voltage_scales_with_sqrt_power() {
    let a = loop_at(1.0, 10.0, 14.0, 1.0);
    let half = magnetic_loop(LoopParams {
      diameter_m: 1.0,
      conductor_mm: 10.0,
      freq_mhz: 14.0,
      turns: 1.0,
      power_w: 25.0,
    })
    .unwrap();
    // P 降到 1/4，电压应降到 1/2。
    assert!((a.cap_voltage / half.cap_voltage - 2.0).abs() < 1e-9);
  }

  #[test]
  fn invalid_inputs_return_none() {
    let p = LoopParams {
      diameter_m: 1.0,
      conductor_mm: 10.0,
      freq_mhz: 14.0,
      turns: 1.0,
      power_w: 100.0,
    };
    assert_eq!(
      magnetic_loop(LoopParams {
        diameter_m: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(
      magnetic_loop(LoopParams {
        conductor_mm: 0.0,
        ..p
      }),
      None
    );
    assert_eq!(magnetic_loop(LoopParams { freq_mhz: 0.0, ..p }), None);
    assert_eq!(magnetic_loop(LoopParams { turns: 0.0, ..p }), None);
    assert_eq!(magnetic_loop(LoopParams { power_w: -1.0, ..p }), None);
    // 匝数必须是正整数：半匝没有物理意义，不能照公式给出一份「1.5 匝环」的参数。
    assert_eq!(magnetic_loop(LoopParams { turns: 0.5, ..p }), None);
    assert_eq!(magnetic_loop(LoopParams { turns: 1.5, ..p }), None);
    assert!(magnetic_loop(LoopParams { turns: 2.0, ..p }).is_some());
    // 环太「粗」：8D/d ≤ e²（D/d ≤ 0.92）时电感公式失效。
    assert_eq!(
      magnetic_loop(LoopParams {
        diameter_m: 0.01,
        conductor_mm: 15.0,
        ..p
      }),
      None
    );
  }
}
