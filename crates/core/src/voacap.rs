//! 点对点 HF 传播预测：基于 foF2 经验模型与太阳黑子数的简化实现。
//!
//! 参考 VOACAP / ITU-R P.533 的思路，给出两点间各业余波段的可用性估计。
//! 为教学与快速判断用途，采用简化公式；结果仅供参考，精确规划请使用专业预测软件。

use serde::Serialize;

use crate::grid::{distance_bearing, lat_lon_from_grid};

/// 业余 HF 波段及其参考频率（MHz）。
pub const HF_BANDS: &[(&str, f64)] = &[
  ("160m", 1.8),
  ("80m", 3.6),
  ("60m", 5.3),
  ("40m", 7.1),
  ("30m", 10.1),
  ("20m", 14.1),
  ("17m", 18.1),
  ("15m", 21.1),
  ("12m", 24.9),
  ("10m", 28.4),
];

/// 单个波段的预测结果。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BandResult {
  /// 波段名，如 `20m`。
  pub band: &'static str,
  /// 参考频率（MHz）。
  pub freq_mhz: f64,
  /// 是否可用（参考频率不超过 MUF）。
  pub usable: bool,
  /// 相对可靠度（0–1）：越接近最佳工作频率（OWF）越高。
  pub reliability: f64,
}

/// 一次点对点预测。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Prediction {
  /// 两点大圆距离（km）。
  pub distance_km: f64,
  /// 初始方位角（度，正北为 0）。
  pub bearing_deg: f64,
  /// 估算的 foF2（MHz）。
  pub fo_f2: f64,
  /// 估算的 MUF（MHz）。
  pub muf: f64,
  /// 各波段结果。
  pub bands: Vec<BandResult>,
}

/// 估算 foF2（MHz）：太阳黑子数越高电离层越强；春秋分略强于冬夏。
#[must_use]
pub fn estimate_fof2(ssn: f64, month: u32) -> f64 {
  let base = 6.0 + 0.02 * ssn;
  let seasonal = 1.0 + 0.08 * (2.0 * std::f64::consts::PI * (month as f64 - 4.0) / 12.0).cos();
  base * seasonal
}

/// 点对点传播预测。网格码非法时返回 `None`。
#[must_use]
pub fn predict(tx_grid: &str, rx_grid: &str, month: u32, ssn: f64) -> Option<Prediction> {
  let (tx_lat, tx_lon) = lat_lon_from_grid(tx_grid)?;
  let (rx_lat, rx_lon) = lat_lon_from_grid(rx_grid)?;
  let (distance_km, bearing_deg) = distance_bearing(tx_lat, tx_lon, rx_lat, rx_lon);

  let fo_f2 = estimate_fof2(ssn, month);
  // 一跳约 3000 km；跳数越多仰角越低，MUF 相应抬升。
  let hops = (distance_km / 3000.0).ceil().max(1.0);
  let muf = fo_f2 * (1.0 + 0.15 * (hops - 1.0));
  let owf = muf * 0.85;

  let bands = HF_BANDS
    .iter()
    .map(|&(band, freq_mhz)| {
      let usable = freq_mhz <= muf;
      let reliability = if freq_mhz <= owf {
        0.5 + 0.5 * (freq_mhz / owf)
      } else if freq_mhz <= muf {
        0.5 * (muf - freq_mhz) / (muf - owf)
      } else {
        0.0
      };
      BandResult {
        band,
        freq_mhz,
        usable,
        reliability,
      }
    })
    .collect();

  Some(Prediction {
    distance_km,
    bearing_deg,
    fo_f2,
    muf,
    bands,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fof2_scales_with_ssn() {
    assert!(estimate_fof2(200.0, 10) > estimate_fof2(0.0, 10));
  }

  #[test]
  fn predict_between_grids() {
    // 北京（OM89ew 附近）→ 伦敦（IO91wm 附近）
    let p = predict("OM89", "IO91", 10, 100.0).expect("valid grids");
    assert!(p.distance_km > 7000.0, "dist {}", p.distance_km);
    assert!(
      (p.bearing_deg - 320.0).abs() < 40.0,
      "bearing {}",
      p.bearing_deg
    );
    // 长距离（约 8150 km）下 MUF 偏低：40m 可用，10m / 20m 通常不可用。
    let forty = p.bands.iter().find(|b| b.band == "40m").unwrap();
    let twenty = p.bands.iter().find(|b| b.band == "20m").unwrap();
    let ten = p.bands.iter().find(|b| b.band == "10m").unwrap();
    assert!(forty.usable);
    assert!(!ten.usable);
    assert!(forty.reliability > ten.reliability);
    assert!(!twenty.usable);
  }

  #[test]
  fn invalid_grid_returns_none() {
    assert_eq!(predict("XX", "IO91", 10, 100.0), None);
    assert_eq!(predict("OM89", "bad", 10, 100.0), None);
  }
}
