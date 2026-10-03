//! 滤波器设计：Butterworth 低通 / 高通 / 带通 / 带阻的集总 L/C 元件值。

use std::f64::consts::TAU;

/// 滤波器类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
  LowPass,
  HighPass,
  BandPass,
  BandStop,
}

/// 单个臂（串联或并联）的元件。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterStage {
  /// 是否串联臂（false 为并联 / 接地臂）。
  pub series: bool,
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
          l_uh: Some(gk * z0 / omega * 1e6),
          c_pf: None,
        }
      } else {
        FilterStage {
          series: false,
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
          l_uh: None,
          c_pf: Some(1.0 / (omega * gk * z0) * 1e12),
        }
      } else {
        FilterStage {
          series: false,
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
        // 低通原型串联电感 → 串联 LC。
        let l = gk * z0 / w_bw;
        let c = bw_hz / (TAU * f0sq * gk * z0);
        FilterStage {
          series: true,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      } else {
        // 低通原型并联电容 → 并联 LC。
        let c = gk / (w_bw * z0);
        let l = bw_hz * z0 / (TAU * f0sq * gk);
        FilterStage {
          series: false,
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
        // 低通原型串联电感 → 并联 LC（与带通的串联 LC 元件值相同，位置对调）。
        let l = gk * z0 / w_bw;
        let c = bw_hz / (TAU * f0sq * gk * z0);
        FilterStage {
          series: false,
          l_uh: Some(l * 1e6),
          c_pf: Some(c * 1e12),
        }
      } else {
        let c = gk / (w_bw * z0);
        let l = bw_hz * z0 / (TAU * f0sq * gk);
        FilterStage {
          series: true,
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
}
