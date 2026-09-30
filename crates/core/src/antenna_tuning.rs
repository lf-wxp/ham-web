//! 天线调试实操：天线分析仪与修剪流程。

/// 调试步骤。
pub const TUNING_STEPS: &[(&str, &str)] = &[
  ("1 初测", "用天线分析仪扫频，找到谐振点与驻波最低频率。"),
  ("2 判断", "谐振频率偏高说明天线偏短，偏低说明偏长。"),
  ("3 修剪", "逐次少量修剪或加长振子，每次后重新扫频。"),
  (
    "4 验证",
    "确认目标频率驻波比 < 2（理想 < 1.5），带宽满足需要。",
  ),
];

/// 调试要点。
pub const TUNING_TIPS: &[&str] = &[
  "修剪留有余量，宁长勿短（加长比截短容易）。",
  "测量时天线置于最终架设高度与位置，避免环境影响误判。",
  "馈线长度也会影响测量，尽量用分析仪直接测天线端。",
  "边测边记，避免来回反复。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn antenna_tuning_data_populated() {
    assert!(TUNING_STEPS.len() >= 3);
    assert!(!TUNING_TIPS.is_empty());
  }
}
