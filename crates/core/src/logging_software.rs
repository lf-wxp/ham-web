//! 日志与竞赛软件：N1MM+、Logger32、HRD 等。

/// 主要软件。
pub const LOGGING_SOFTWARE: &[(&str, &str, &str)] = &[
  (
    "N1MM Logger+",
    "竞赛日志",
    "最流行的竞赛日志软件，支持各种竞赛规则与 DX Cluster。",
  ),
  ("Logger32", "通用日志", "老牌通用日志软件，功能全面。"),
  (
    "HRD Logbook",
    "Ham Radio Deluxe",
    "集成日志、遥控、数字模式的套件。",
  ),
  (
    "WSJT-X",
    "弱信号模式",
    "FT8/FT4 收发，与日志软件联动自动记录。",
  ),
];

/// 使用要点。
pub const SOFTWARE_NOTES: &[&str] = &[
  "竞赛使用 N1MM+ 可自动评分、控制电台、连接 DX Cluster。",
  "日常日志可导出 ADIF，导入 LoTW/eQSL 等确认。",
  "WSJT-X 的 UDP 广播可与多数日志软件自动联动记录。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn logging_software_populated() {
    assert!(LOGGING_SOFTWARE.len() >= 3);
    assert!(!SOFTWARE_NOTES.is_empty());
  }
}
