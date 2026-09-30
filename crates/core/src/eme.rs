//! EME 月面反射：天线阵列、功率与操作。

/// 核心概念。
pub const EME_CONCEPTS: &[(&str, &str)] = &[
  (
    "EME",
    "Earth-Moon-Earth，利用月球反射电波，实现地球任意两地通联。",
  ),
  ("路径损耗", "往返约 250dB+，是业余通信中最苛刻的传播方式。"),
  ("月面窗口", "按月球位置与多普勒计算最佳通联窗口。"),
];

/// 设备要求。
pub const EME_REQUIREMENTS: &[(&str, &str, &str)] = &[
  ("天线", "高增益阵列", "多个八木组成阵列或大型抛物面。"),
  ("功率", "数十至数百瓦", "需大功率功放与低损耗馈线。"),
  (
    "模式",
    "JT65 等弱信号模式",
    "现代多用 JT65/Q65 等数字模式降低要求。",
  ),
];

/// 操作要点。
pub const EME_TIPS: &[&str] = &[
  "现代 EME 用 JT65/Q65 数字模式，小台站也能参与。",
  "2m EME 用 144MHz，是最热门的 EME 波段。",
  "精确对准月球并补偿多普勒频率漂移。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eme_data_populated() {
    assert!(!EME_CONCEPTS.is_empty());
    assert!(EME_REQUIREMENTS.len() >= 3);
    assert!(!EME_TIPS.is_empty());
  }
}
