//! 通联实务速查：通联流程、日志字段、QSL 卡片、中继台使用、接地与防雷。

/// 一次标准通联的流程。
pub const CONTACT_STEPS: &[(&str, &str)] = &[
  (
    "发起呼叫",
    "CQ CQ CQ, this is BG4XXX calling CQ and standing by.",
  ),
  (
    "应答确认",
    "对方回应后，报双方呼号并给出信号报告（如 59）。",
  ),
  ("交换信息", "简短交换姓名、QTH 位置、设备等信息。"),
  ("通联结束", "致 73，再次报双方呼号，结束并转入守听。"),
];

/// 通联日志应记录的字段。
pub const LOG_FIELDS: &[(&str, &str)] = &[
  ("日期", "通联日期（建议用 UTC）"),
  ("时间", "通联开始时间（UTC）"),
  ("频率", "本次通联所用频率"),
  ("模式", "CW / SSB / FM / FT8 等"),
  ("对方呼号", "通联对象呼号"),
  ("信号报告", "RST 或 RS 报告"),
  ("备注", "对方 QTH、设备、姓名等"),
];

/// QSL 卡片应包含的信息。
pub const QSL_FIELDS: &[(&str, &str)] = &[
  ("双方呼号", "本台呼号与对方呼号"),
  ("确认内容", "日期、时间、频率、模式、信号报告"),
  ("双方信息", "操作员姓名、QTH、设备"),
  ("签名", "操作员签名确认"),
];

/// 中继台使用要点。
pub const REPEATER_TIPS: &[&str] = &[
  "中继台上下行异频：上行是你的发射频率，下行是收听频率，二者之差为频差。",
  "多数中继台发射时需加亚音（CTCSS），否则无法打开中继。",
  "上台先报呼号，通联简短，留出间隔让其他台插入。",
  "VHF 频差通常 0.6 MHz，UHF 通常 5 MHz。",
];

/// 接地与防雷要点。
pub const GROUNDING_TIPS: &[&str] = &[
  "电台、馈线与天线的良好接地是防雷与安全的基础。",
  "馈线进入室内前应加装避雷器并就近接地。",
  "雷电天气应断开天线与电源，切勿操作电台。",
  "天线应远离架空电力线，防止触电与感应雷击。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn operating_data_populated() {
    assert!(!CONTACT_STEPS.is_empty());
    assert!(!LOG_FIELDS.is_empty());
    assert!(!QSL_FIELDS.is_empty());
    assert!(!REPEATER_TIPS.is_empty());
    assert!(!GROUNDING_TIPS.is_empty());
  }
}
