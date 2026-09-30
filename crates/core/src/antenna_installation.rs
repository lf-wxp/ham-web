//! 天线架设与支撑：高度、拉线、桅杆/塔与安全。

/// 支撑方式。
pub const SUPPORT_TYPES: &[(&str, &str, &str)] = &[
  (
    "便携撑杆",
    "数米，玻璃钢/铝管",
    "临时架设，SOTA/POTA 常用。",
  ),
  ("桅杆", "10–20m，拉线固定", "基地台常用，倒 V、偶极天线。"),
  ("铁塔", "20m+，需基础与拉线", "大型八木、高增益定向天线。"),
];

/// 架设要点。
pub const INSTALL_TIPS: &[&str] = &[
  "高度决定覆盖：VHF/UHF 天线越高越远，HF 则按波段选择合适高度。",
  "拉线用绝缘子分段，避免拉线感应电流影响天线方向图。",
  "馈线进入室内前加装避雷器并就近接地。",
  "架设远离电力线，雷雨天气停止操作并断开天线。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn installation_data_populated() {
    assert!(SUPPORT_TYPES.len() >= 3);
    assert!(!INSTALL_TIPS.is_empty());
  }
}
