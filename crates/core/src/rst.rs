//! RST 信号报告：可懂度（R）、信号强度（S）、音调（T）与常用报告示例。

/// R（Readability 可懂度）1–5。
pub const READABILITY: &[(&str, &str)] = &[
  ("R1", "无法辨别"),
  ("R2", "偶尔可辨个别字"),
  ("R3", "有相当困难才能辨别"),
  ("R4", "基本可辨，略有困难"),
  ("R5", "完全清晰可辨"),
];

/// S（Strength 信号强度）S1–S9（S9 对应接收机输入端约 50 μV / 50 Ω）。
pub const SIGNAL_STRENGTH: &[(&str, &str)] = &[
  ("S1", "极微弱信号"),
  ("S2", "很弱信号"),
  ("S3", "弱信号"),
  ("S4", "较弱信号"),
  ("S5", "中等信号"),
  ("S6", "中等偏强"),
  ("S7", "较强信号"),
  ("S8", "很强信号"),
  ("S9", "极强信号"),
];

/// T（Tone 音调）1–9，仅用于 CW。
pub const TONE: &[(&str, &str)] = &[
  ("T1", "极粗糙的交流声"),
  ("T2", "很粗糙的交流声"),
  ("T3", "粗糙、带交流声"),
  ("T4", "略带粗糙"),
  ("T5", "轻微滤波的交流声"),
  ("T6", "滤波音，略有波纹"),
  ("T7", "接近纯音，稍有波纹"),
  ("T8", "接近纯音"),
  ("T9", "纯净音调"),
];

/// 常用报告示例与说明。
pub const RST_EXAMPLES: &[(&str, &str)] = &[
  ("59", "语音通联的完美报告（R5 + S9）"),
  ("599", "CW 通联的完美报告（R5 + S9 + T9）"),
  ("RS(T)", "语音报 R+S 两位，CW 报 R+S+T 三位"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rst_tables_populated() {
    assert_eq!(READABILITY.len(), 5);
    assert_eq!(SIGNAL_STRENGTH.len(), 9);
    assert_eq!(TONE.len(), 9);
    assert!(!RST_EXAMPLES.is_empty());
  }
}
