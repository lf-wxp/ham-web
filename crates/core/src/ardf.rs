//! 无线电测向 ARDF：测向原理与竞赛频段。

/// 核心概念。
pub const ARDF_CONCEPTS: &[(&str, &str)] = &[
  (
    "ARDF",
    "Amateur Radio Direction Finding，业余无线电测向，俗称「猎狐」。",
  ),
  (
    "测向原理",
    "利用指向性天线（如环形天线、八木）判断信号来向，用三角定位确定位置。",
  ),
  (
    "隐蔽电台",
    "竞赛中隐藏发射信标的电台，选手依据信号找到它们。",
  ),
  (
    "测向天线",
    "环形天线双向性、加垂直振子合成心形方向图，可辨单向。",
  ),
];

/// 常用频段。
pub const ARDF_BANDS: &[(&str, &str)] = &[
  ("2m（144MHz）", "常用测向频段，天线尺寸小、方向性好。"),
  ("80m（3.5MHz）", "长波测向，需较大环形天线。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ardf_data_populated() {
    assert!(!ARDF_CONCEPTS.is_empty());
    assert!(!ARDF_BANDS.is_empty());
  }
}
