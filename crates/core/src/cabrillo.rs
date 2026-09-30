//! Cabrillo 竞赛日志：格式、记分与提交。

/// 格式结构。
pub const CABRILLO_CONCEPTS: &[(&str, &str)] = &[
  ("Cabrillo", "业余无线电竞赛提交日志的标准纯文本格式。"),
  (
    "头信息",
    "以 START-OF-LOG / END-OF-LOG 包裹，含台站与比赛信息。",
  ),
  (
    "QSO 行",
    "每条记录：频率、模式、日期、时间、对方呼号、交换信息。",
  ),
  ("记分", "按规则统计通联数、系数（分区 / 前缀 / 州）与总分。"),
];

/// 提交要点。
pub const CABRILLO_TIPS: &[&str] = &[
  "用日志软件（N1MM、DXLog 等）自动导出 Cabrillo，避免手工出错。",
  "提交前核对呼号、交换信息与时间，格式错误会被拒收。",
  "在比赛官网指定截止时间前提交日志。",
  "保留原始日志，便于申诉或核对通联有效性。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cabrillo_data_populated() {
    assert!(CABRILLO_CONCEPTS.len() >= 4);
    assert!(!CABRILLO_TIPS.is_empty());
  }
}
