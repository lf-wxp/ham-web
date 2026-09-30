//! 电子 QSL 与日志确认：LoTW、eQSL、QRZ、Club Log 等现代确认方式。

/// 主要服务。
pub const EQSL_SERVICES: &[(&str, &str, &str)] = &[
  (
    "LoTW",
    "ARRL 维护，数字签名日志",
    "DXCC 等奖状的主要确认方式，需 TQSL 证书签名上传。",
  ),
  ("eQSL", "电子 QSL 卡片", "电子卡片交换，与纸质 QSL 类似。"),
  (
    "QRZ Logbook",
    "QRZ.com 日志",
    "在线日志与 QSL 管理，社区广泛使用。",
  ),
  (
    "Club Log",
    "日志分析平台",
    "传播分析、稀有台需求匹配与 OQRS 卡片请求。",
  ),
];

/// 说明要点。
pub const EQSL_NOTES: &[&str] = &[
  "电子确认通常比纸质 QSL 更快捷，是现代奖状申请的主流方式。",
  "LoTW 需要 TQSL 证书并签名上传，可复用通联日志导出的 ADIF 文件。",
  "纸质 QSL 仍有纪念意义，可通过 OQRS 或直邮 + 回邮券交换。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eqsl_data_populated() {
    assert!(EQSL_SERVICES.len() >= 3);
    assert!(!EQSL_NOTES.is_empty());
  }
}
