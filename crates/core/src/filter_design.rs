//! 滤波器设计：Butterworth 低通 / 高通 / 带通 / 带阻的集总 L/C 元件值。
//!
//! 注：带阻（陷波）采用与带通对偶的串 / 并位置，各级谐振于中心频率；谐振器在臂内的
//! 连接方式见工具页说明。幅频响应（S21）由单元测试
//! `bandstop_transfer_function_is_a_notch` / `bandpass_transfer_function_is_a_passband`
//! 交叉验证（带阻须为阻带、带通须为通带），不再只依赖元件谐振与带边电抗。

use std::f64::consts::TAU;

/// 滤波器类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
  LowPass,
  HighPass,
  BandPass,
  BandStop,
}

/// 臂内谐振器的连接方式。
///
/// 低通 / 高通的臂内只有一个元件；带通与带阻的臂内是一个谐振器，必须区分串并联，
/// 否则装配出来的拓扑完全不同（把带阻的并联 LC 装成串联 LC，得到的是带通）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResonatorTopology {
  /// 单个 L 或 C（低通 / 高通）。
  Single,
  /// 串联 LC：谐振时阻抗最小（近似短路）。
  SeriesLc,
  /// 并联 LC：谐振时阻抗最大（近似开路）。
  ParallelLc,
}

/// 单个臂（串联或并联）的元件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterStage {
  /// 是否串联臂（false 为并联 / 接地臂）。
  pub series: bool,
  /// 臂内谐振器的连接方式。
  pub topology: ResonatorTopology,
  /// 电感（μH）。
  pub l_uh: Option<f64>,
  /// 电容（pF）。
  pub c_pf: Option<f64>,
}

/// Butterworth 归一化低通原型 g 值（g₀ 与 g_{n+1} 隐含为 1）。
pub fn butterworth_g(n: usize) -> Option<&'static [f64]> {
  match n {
    1 => Some(&[2.0]),
    2 => Some(&[std::f64::consts::SQRT_2, std::f64::consts::SQRT_2]),
    3 => Some(&[1.0, 2.0, 1.0]),
    4 => Some(&[
      0.765_366_864_7,
      1.847_759_065_0,
      1.847_759_065_0,
      0.765_366_864_7,
    ]),
    5 => Some(&[
      0.618_033_988_7,
      1.618_033_988_7,
      2.0,
      1.618_033_988_7,
      0.618_033_988_7,
    ]),
    _ => None,
  }
}

/// 设计低通滤波器（截止频率 fc_hz，特性阻抗 z0）。
pub fn design_lowpass(n: usize, fc_hz: f64, z0: f64) -> Vec<FilterStage> {
  let Some(g) = butterworth_g(n) else {
    return Vec::new();
  };
  let omega = TAU * fc_hz;
  g.iter()
    .enumerate()
    .map(|(k, &gk)| {
      if k % 2 == 0 {
        FilterStage {
          series: true,
          topology: ResonatorTopology::Single,
          l_uh: Some(gk * z0 / omega * 1e6),
          c_pf: None,
        }
      } else {
        FilterStage {
          series: false,
          topology: ResonatorTopology::Single,
          l_uh: None,
          c_pf: Some(gk / (omega * z0) * 1e12),
        }
      }
    })
    .collect()
}

/// 设计高通滤波器（截止频率 fc_hz，特性阻抗 z0）。
pub fn design_highpass(n: usize, fc_hz: f64, z0: f64) -> Vec<FilterStage> {
  let Some(g) = butterworth_g(n) else {
    return Vec::new();
  };
  let omega = TAU * fc_hz;
  g.iter()
    .enumerate()
    .map(|(k, &gk)| {
      if k % 2 == 0 {
        FilterStage {
          series: true,
          topology: ResonatorTopology::Single,
          l_uh: None,
          c_pf: Some(1.0 / (omega * gk * z0) * 1e12),
        }
      } else {
        FilterStage {
          series: false,
          topology: ResonatorTopology::Single,
          l_uh: Some(z0 / (omega * gk) * 1e6),
          c_pf: None,
        }
      }
    })
    .collect()
}

/// 设计带通滤波器（中心 f0_hz，带宽 bw_hz，特性阻抗 z0）。
pub fn design_bandpass(n: usize, f0_hz: f64, bw_hz: f64, z0: f64) -> Vec<FilterStage> {
  let Some(g) = butterworth_g(n) else {
    return Vec::new();
  };
  let w_bw = TAU * bw_hz;
  let f0sq = f0_hz * f0_hz;
  g.iter()
    .enumerate()
    .map(|(k, &gk)| {
      if k % 2 == 0 {
        // 低通原型串联电感 → 串联 LC（谐振时短路，串联臂导通 = 通带）。
        let l = gk * z0 / w_bw;
        let c = bw_hz / (TAU * f0sq * gk * z0);
        FilterStage {
          series: true,
          topology: ResonatorTopology::SeriesLc,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      } else {
        // 低通原型并联电容 → 并联 LC（谐振时开路，旁路臂失效 = 通带）。
        let c = gk / (w_bw * z0);
        let l = bw_hz * z0 / (TAU * f0sq * gk);
        FilterStage {
          series: false,
          topology: ResonatorTopology::ParallelLc,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      }
    })
    .collect()
}

/// 设计带阻（陷波）滤波器（中心 f0_hz，带宽 bw_hz，特性阻抗 z0）。
pub fn design_bandstop(n: usize, f0_hz: f64, bw_hz: f64, z0: f64) -> Vec<FilterStage> {
  let Some(g) = butterworth_g(n) else {
    return Vec::new();
  };
  let w_bw = TAU * bw_hz;
  let f0sq = f0_hz * f0_hz;
  g.iter()
    .enumerate()
    .map(|(k, &gk)| {
      if k % 2 == 0 {
        // 带阻取带通的对偶（串 / 并位置互换）：并联（接地）臂内为串联 LC，
        // 谐振时短路，把信号旁路到地 = 阻带。
        //
        // 带阻变换 s → Δω·s/(s²+ω0²) 作用于原型元件，得臂内串联 LC：
        // L = Z₀/(g·Δω)、C = g·Δω/(ω0²·Z₀)。
        // 与带通的区别不只是位置对调 —— g 的位置相反（带通 L∝g，带阻 L∝1/g）。
        let l = z0 / (gk * w_bw);
        let c = gk * bw_hz / (TAU * f0sq * z0);
        FilterStage {
          series: false,
          topology: ResonatorTopology::SeriesLc,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      } else {
        // 对偶的另一半：串联臂内为并联 LC，谐振时开路，串在通路里 = 阻带。
        let c = 1.0 / (gk * w_bw * z0);
        let l = gk * bw_hz * z0 / (TAU * f0sq);
        FilterStage {
          series: true,
          topology: ResonatorTopology::ParallelLc,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lowpass_stage_count_and_alternation() {
    let stages = design_lowpass(3, 7.1e6, 50.0);
    assert_eq!(stages.len(), 3);
    assert!(stages[0].series);
    assert!(stages[0].l_uh.is_some());
    assert!(!stages[1].series);
    assert!(stages[1].c_pf.is_some());
    assert!(stages[2].series);
  }

  #[test]
  fn highpass_first_stage_is_series_capacitor() {
    let stages = design_highpass(2, 7.1e6, 50.0);
    assert_eq!(stages.len(), 2);
    assert!(stages[0].series);
    assert!(stages[0].c_pf.is_some());
  }

  /// 上边频（几何中心口径）。
  ///
  /// 频率变换要求 f₁·f₂ = f₀²、f₂ − f₁ = BW。若直接取 f₀ ± BW/2（算术中心），
  /// 会引入约 BW/(4f₀) 的窄带近似误差，掩盖真正的元件值错误。
  fn upper_band_edge_hz(f0: f64, bw: f64) -> f64 {
    (bw + (bw * bw + 4.0 * f0 * f0).sqrt()) / 2.0
  }

  /// 带边归一化电（导）抗：Butterworth 原型要求它在上边频处等于 gₖ。
  ///
  /// 只看 L·C 乘积（= 1/ω0²）抓不到元件值错误 —— 把 gₖ 与 1/gₖ 用反时乘积不变。
  /// 串联臂取 |z|/Z₀，并联臂取 |y|·Z₀，两者都应等于 gₖ。
  fn band_edge_normalized(stage: &FilterStage, w_upper: f64, z0: f64) -> f64 {
    let l = stage.l_uh.expect("L") * 1e-6;
    let c = stage.c_pf.expect("C") * 1e-12;
    // 串联 LC 天然给出 z，并联 LC 天然给出 y。
    let natural = match stage.topology {
      ResonatorTopology::SeriesLc => (w_upper * l - 1.0 / (w_upper * c)).abs(),
      ResonatorTopology::ParallelLc => (w_upper * c - 1.0 / (w_upper * l)).abs(),
      ResonatorTopology::Single => f64::NAN,
    };
    // 串联臂要的是 |z|/Z₀，并联臂要的是 |y|·Z₀；拓扑给的正好相反时取倒数。
    match (stage.series, stage.topology) {
      (true, ResonatorTopology::SeriesLc) => natural / z0,
      (true, ResonatorTopology::ParallelLc) => 1.0 / (natural * z0),
      (false, ResonatorTopology::ParallelLc) => natural * z0,
      (false, ResonatorTopology::SeriesLc) => z0 / natural,
      (_, ResonatorTopology::Single) => f64::NAN,
    }
  }

  #[test]
  fn bandpass_band_edge_reactance_matches_g() {
    let f0 = 14.2e6;
    let bw = 0.5e6;
    let z0 = 50.0;
    let w_upper = TAU * upper_band_edge_hz(f0, bw);
    let g = butterworth_g(3).unwrap();
    for (stage, &gk) in design_bandpass(3, f0, bw, z0).iter().zip(g.iter()) {
      let got = band_edge_normalized(stage, w_upper, z0);
      assert!(
        (got - gk).abs() / gk < 1e-6,
        "带通带边归一化电抗应等于 gₖ={gk}，实际 {got}"
      );
    }
  }

  #[test]
  fn bandstop_band_edge_reactance_matches_g() {
    let f0 = 7.1e6;
    let bw = 0.5e6;
    let z0 = 50.0;
    let w_upper = TAU * upper_band_edge_hz(f0, bw);
    let stages = design_bandstop(3, f0, bw, z0);
    let g = butterworth_g(3).unwrap();
    for (stage, &gk) in stages.iter().zip(g.iter()) {
      let got = band_edge_normalized(stage, w_upper, z0);
      assert!(
        (got - gk).abs() / gk < 1e-6,
        "带阻带边归一化电抗应等于 gₖ={gk}，实际 {got}（gₖ 与 1/gₖ 用反时会差 gₖ² 倍）"
      );
    }
    // 中间级 g=2：错误值会是 0.5（差 4 倍），这里显式钉住。
    assert!(
      (band_edge_normalized(&stages[1], w_upper, z0) - 2.0).abs() < 1e-6,
      "带阻第二级 g=2"
    );
  }

  #[test]
  fn bandstop_arms_have_inverted_topology() {
    let stages = design_bandstop(3, 7.1e6, 0.5e6, 50.0);
    // 并联臂内是串联 LC（谐振短路→旁路到地），串联臂内是并联 LC（谐振开路→阻断）。
    assert!(!stages[0].series && stages[0].topology == ResonatorTopology::SeriesLc);
    assert!(stages[1].series && stages[1].topology == ResonatorTopology::ParallelLc);
  }

  #[test]
  fn bandpass_lc_resonates_at_center() {
    let f0 = 14.2e6;
    let stages = design_bandpass(3, f0, 0.5e6, 50.0);
    for s in stages {
      let l = s.l_uh.expect("bandpass stage should have L") * 1e-6;
      let c = s.c_pf.expect("bandpass stage should have C") * 1e-12;
      let resonant = 1.0 / (TAU * (l * c).sqrt());
      assert!((resonant - f0).abs() / f0 < 1e-6);
    }
  }

  #[test]
  fn bandstop_lc_resonates_at_center() {
    let f0 = 3.6e6;
    let stages = design_bandstop(2, f0, 0.3e6, 50.0);
    for s in stages {
      let l = s.l_uh.expect("bandstop stage should have L") * 1e-6;
      let c = s.c_pf.expect("bandstop stage should have C") * 1e-12;
      let resonant = 1.0 / (TAU * (l * c).sqrt());
      assert!((resonant - f0).abs() / f0 < 1e-6);
    }
  }

  // ---------------------------------------------------------------------------
  // S21 传输函数级回归：验证幅频响应确实是「通带 / 阻带」，而不只是元件谐振与
  // 带边电抗正确 —— 后者无法证明臂的位置（串联 / 并联）没接反。
  // ---------------------------------------------------------------------------

  /// 极简复数（仅测试用，避免引入额外依赖）。方法名刻意避开 `std::ops` 的
  /// `add` / `mul` / `div` / `from`，否则会触发 `clippy::should_implement_trait`。
  #[derive(Clone, Copy)]
  struct Cx(f64, f64);

  impl Cx {
    const ONE: Cx = Cx(1.0, 0.0);
    const ZERO: Cx = Cx(0.0, 0.0);
    fn of(v: f64) -> Cx {
      Cx(v, 0.0)
    }
    fn jw(w: f64) -> Cx {
      Cx(0.0, w)
    }
    fn plus(self, o: Cx) -> Cx {
      Cx(self.0 + o.0, self.1 + o.1)
    }
    fn times(self, o: Cx) -> Cx {
      Cx(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn over(self, o: Cx) -> Cx {
      let d = o.0 * o.0 + o.1 * o.1;
      Cx(
        (self.0 * o.0 + self.1 * o.1) / d,
        (self.1 * o.0 - self.0 * o.1) / d,
      )
    }
    fn inv(self) -> Cx {
      Cx::ONE.over(self)
    }
    fn abs(self) -> f64 {
      self.0.hypot(self.1)
    }
  }

  /// 单臂的复阻抗（串联臂）或复导纳（并联臂）。
  fn arm_x(stage: &FilterStage, w: f64) -> Cx {
    let jw = Cx::jw(w);
    // 元件阻抗：z_L = jωL、z_C = 1/(jωC)。
    if stage.series {
      match stage.topology {
        ResonatorTopology::Single => {
          if let Some(l) = stage.l_uh {
            jw.times(Cx::of(l * 1e-6))
          } else {
            jw.times(Cx::of(stage.c_pf.expect("C") * 1e-12)).inv()
          }
        }
        ResonatorTopology::SeriesLc => {
          let zl = jw.times(Cx::of(stage.l_uh.expect("L") * 1e-6));
          let zc = jw.times(Cx::of(stage.c_pf.expect("C") * 1e-12)).inv();
          zl.plus(zc)
        }
        ResonatorTopology::ParallelLc => {
          let yl = jw.times(Cx::of(stage.l_uh.expect("L") * 1e-6)).inv();
          let yc = jw.times(Cx::of(stage.c_pf.expect("C") * 1e-12));
          yl.plus(yc).inv()
        }
      }
    } else {
      match stage.topology {
        ResonatorTopology::Single => {
          if let Some(c) = stage.c_pf {
            jw.times(Cx::of(c * 1e-12))
          } else {
            jw.times(Cx::of(stage.l_uh.expect("L") * 1e-6)).inv()
          }
        }
        ResonatorTopology::ParallelLc => {
          let yc = jw.times(Cx::of(stage.c_pf.expect("C") * 1e-12));
          let yl = jw.times(Cx::of(stage.l_uh.expect("L") * 1e-6)).inv();
          yc.plus(yl)
        }
        ResonatorTopology::SeriesLc => {
          let zl = jw.times(Cx::of(stage.l_uh.expect("L") * 1e-6));
          let zc = jw.times(Cx::of(stage.c_pf.expect("C") * 1e-12)).inv();
          zl.plus(zc).inv()
        }
      }
    }
  }

  /// 单个二端口 ABCD 矩阵，行主序 `[A, B, C, D]`。
  fn arm_abcd(series: bool, x: Cx) -> [Cx; 4] {
    if series {
      [Cx::ONE, x, Cx::ZERO, Cx::ONE]
    } else {
      [Cx::ONE, Cx::ZERO, x, Cx::ONE]
    }
  }

  fn mat_mul(m: [Cx; 4], n: [Cx; 4]) -> [Cx; 4] {
    let [a1, b1, c1, d1] = m;
    let [a2, b2, c2, d2] = n;
    [
      a1.times(a2).plus(b1.times(c2)),
      a1.times(b2).plus(b1.times(d2)),
      c1.times(a2).plus(d1.times(c2)),
      c1.times(b2).plus(d1.times(d2)),
    ]
  }

  /// 阶梯网络在 `freq_hz` 处的 S21（dB）：级联各臂 ABCD，再按二端口公式换算。
  fn s21_db(stages: &[FilterStage], freq_hz: f64, z0: f64) -> f64 {
    let w = TAU * freq_hz;
    let mut m = [Cx::ONE, Cx::ZERO, Cx::ZERO, Cx::ONE];
    for stage in stages {
      m = mat_mul(m, arm_abcd(stage.series, arm_x(stage, w)));
    }
    let [a, b, c, d] = m;
    let denom = a.plus(b.over(Cx::of(z0))).plus(c.times(Cx::of(z0))).plus(d);
    let s21 = Cx::of(2.0).over(denom).abs();
    20.0 * s21.log10()
  }

  /// 带阻的幅频响应必须是「阻带」：中心深衰、阻带外低插损、带边 ≈ −3dB。
  #[test]
  fn bandstop_transfer_function_is_a_notch() {
    let (f0, bw, z0) = (7.1e6, 0.5e6, 50.0);
    let stages = design_bandstop(3, f0, bw, z0);
    // 中心偏 1e-6 一点，避开谐振点处必然出现的 0/0。
    let at_center = s21_db(&stages, f0 * 1.000_001, z0);
    let below = s21_db(&stages, f0 * 0.5, z0);
    let above = s21_db(&stages, f0 * 2.0, z0);
    let edge = s21_db(&stages, upper_band_edge_hz(f0, bw), z0);
    assert!(at_center < -30.0, "带阻中心应深衰：{at_center} dB");
    assert!(
      below > -1.0 && above > -1.0,
      "阻带外应低插损：{below} / {above} dB"
    );
    assert!((edge + 3.0103).abs() < 0.3, "带边应约 −3dB：{edge} dB");
  }

  /// 对照组：同样用 S21 判据看带通 —— 中心直通、通带外阻断。用同一判据能把
  /// 带通与带阻区分开，才说明上面那条测试真的在测「阻带」而不是碰巧成立。
  #[test]
  fn bandpass_transfer_function_is_a_passband() {
    let (f0, bw, z0) = (14.2e6, 0.5e6, 50.0);
    let stages = design_bandpass(3, f0, bw, z0);
    let at_center = s21_db(&stages, f0, z0);
    let low = s21_db(&stages, f0 * 0.25, z0);
    let high = s21_db(&stages, f0 * 4.0, z0);
    let edge = s21_db(&stages, upper_band_edge_hz(f0, bw), z0);
    assert!(at_center > -1.0, "带通中心应低插损：{at_center} dB");
    assert!(
      low < -20.0 && high < -20.0,
      "通带外应深衰：{low} / {high} dB"
    );
    assert!((edge + 3.0103).abs() < 0.3, "带边应约 −3dB：{edge} dB");
  }
}
