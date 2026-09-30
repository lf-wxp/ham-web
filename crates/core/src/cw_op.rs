//! CW 操作规范与常用缩语：CW 通联中的专用缩语与礼仪。

/// 常用 CW 缩语（区别于 Q 简语）。
pub const CW_ABBREVIATIONS: &[(&str, &str)] = &[
  ("DE", "from，来自（分隔双方呼号，如 BG4XYZ DE JA1ABC）"),
  ("K", "over，请讲（邀请对方发射）"),
  ("BK", "break，打断/快速切换收发"),
  ("R", "收到，Roger"),
  ("73", "祝福，best regards"),
  ("88", "祝福（对女士，love and kisses）"),
  ("SK", "结束通联，silent key（也指已故台友）"),
  ("CQ", "普遍呼叫"),
  ("RST", "信号报告（可懂度/强度/音调）"),
  ("QRL?", "频率是否被占用（Q 简语）"),
];

/// 操作要点。
pub const CW_TIPS: &[&str] = &[
  "发送前先听，确认频率空闲再呼叫。",
  "报呼号至少两遍，速度以对方能抄收为准。",
  "对方发送时耐心等待，使用 K/BK 正确交接。",
  "结束用 73 或 SK，简洁清晰。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cw_op_data_populated() {
    assert!(CW_ABBREVIATIONS.len() >= 6);
    assert!(!CW_TIPS.is_empty());
  }
}
