//! 中继台建设与维护：选址、双工器、覆盖与日常维护。

/// 建设要素。
pub const REPEATER_BUILD: &[(&str, &str)] = &[
  ("选址", "选高处以扩大覆盖，兼顾电源、天线与馈线条件。"),
  ("双工器", "收发共用天线，需足够收发隔离度（通常 ≥ 80dB）。"),
  ("天线", "高增益全向或定向天线，馈线损耗要小。"),
  ("电源", "UPS + 电池，保证停电仍能工作。"),
  (
    "频率",
    "按当地频率协调结果设定输入 / 输出频差（如 ±5MHz）。",
  ),
];

/// 维护要点。
pub const REPEATER_MAINT: &[&str] = &[
  "定期检测输出功率、驻波比与接收灵敏度。",
  "记录通联日志与故障，便于追溯与排障。",
  "做好防雷接地，雷雨季节前重点检查。",
  "遵守频率协调与执照要求，避免干扰其他台站。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn repeater_build_data_populated() {
    assert!(REPEATER_BUILD.len() >= 5);
    assert!(!REPEATER_MAINT.is_empty());
  }
}
