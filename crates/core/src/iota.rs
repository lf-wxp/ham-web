//! IOTA 海岛通联：Islands On The Air 奖状与岛组。

/// 核心概念。
pub const IOTA_CONCEPTS: &[(&str, &str)] = &[
  (
    "IOTA",
    "Islands On The Air，英国 RSGB 主办的海岛通联奖状计划。",
  ),
  (
    "岛组编号",
    "全球海岛按地理分组编号，如 AS-xxx（亚洲）、OC-xxx（大洋洲）。",
  ),
  ("激活者", "登岛设台的电台，可激活岛组并积累积分。"),
  ("追逐者", "与海岛台通联以收集岛组，满足奖状要求。"),
  ("奖状", "通联规定数量的岛组可获得 IOTA 奖状与奖牌。"),
];

/// 参与要点。
pub const IOTA_TIPS: &[&str] = &[
  "海岛台呼号常带 /P（便携/登岛）；/MM 表示海上移动，不属海岛设台。通联前确认其所在岛组编号。",
  "岛组编号与 DXCC 实体不同，同一实体可能有多个岛组。",
  "稀有岛组远征同样会形成 pileup，遵守分裂操作礼仪。",
  "记录岛组编号，配合日志软件与奖状追踪工具。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn iota_data_populated() {
    assert!(IOTA_CONCEPTS.len() >= 4);
    assert!(!IOTA_TIPS.is_empty());
  }
}
