//! 微波通信：10GHz 及以上频段、抛物面天线与操作。

/// 微波频段。
pub const MICROWAVE_BANDS: &[(&str, &str, &str)] = &[
  (
    "3cm 波段",
    "10.0–10.5 GHz",
    "最活跃的业余微波段，抛物面天线为主。",
  ),
  ("1.2cm 波段", "24.0–24.25 GHz", "窄波束，适合点对点与比赛。"),
  ("6mm 波段", "47.0–47.2 GHz", "毫米波，带宽大，距离受限。"),
];

/// 核心概念。
pub const MICROWAVE_CONCEPTS: &[(&str, &str)] = &[
  (
    "抛物面天线",
    "用抛物面反射器聚焦电波，增益高、波束窄，微波通信核心。",
  ),
  ("波导", "微波段馈线损耗大，常用波导代替同轴电缆。"),
  (
    "对准",
    "波束极窄，需精确对准对方方向，通常配合信标与功率计。",
  ),
];

/// 操作要点。
pub const MICROWAVE_TIPS: &[&str] = &[
  "先对准再发射，用对方信标或场强仪辅助对准。",
  "短距离（数十 km）用小型抛物面即可，长距离需更大口径。",
  "微波多用于本地竞赛与实验，注意功率与安全。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn microwave_data_populated() {
    assert!(MICROWAVE_BANDS.len() >= 3);
    assert!(!MICROWAVE_CONCEPTS.is_empty());
    assert!(!MICROWAVE_TIPS.is_empty());
  }
}
