//! 天线极化：水平、垂直与圆极化。

/// 极化类型。
pub const POLARIZATION_TYPES: &[(&str, &str)] = &[
  ("水平极化", "电场方向与地面平行，HF 偶极、八木常用。"),
  ("垂直极化", "电场方向垂直地面，车载天线、手持机常用。"),
  ("圆极化", "电场矢量旋转，用于卫星通信（RHCP / LHCP）。"),
  ("椭圆极化", "介于线极化与圆极化之间的一般情况。"),
];

/// 选择与匹配要点。
pub const POLARIZATION_TIPS: &[&str] = &[
  "收发极化不一致会产生极化损耗：正交线极化损耗极大（理论 20dB+）。",
  "HF 天波经电离层旋转，极化影响较小；VHF / UHF 视距影响显著。",
  "卫星通信多用圆极化，需注意左右旋（RHCP / LHCP）匹配。",
  "移动台多为垂直极化，与基地水平极化天线通联会有损耗。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn polarization_data_populated() {
    assert!(POLARIZATION_TYPES.len() >= 4);
    assert!(!POLARIZATION_TIPS.is_empty());
  }
}
