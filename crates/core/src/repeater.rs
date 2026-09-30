//! 中继台与数字网关：中继台原理、数字中继、热点与互联网网关。

/// 中继台概念。
pub const REPEATER_CONCEPTS: &[(&str, &str)] = &[
  (
    "异频中继",
    "中继台用上行频率接收、下行频率发射，同时转发，扩大覆盖。",
  ),
  ("频差", "上下行频率之差：VHF 通常 0.6MHz，UHF 通常 5MHz。"),
  (
    "亚音 CTCSS",
    "发射时叠加亚音频，只有带匹配亚音的台才能打开中继，避免误触发。",
  ),
  ("回波", "中继台发射时会回传自身信号，用于确认是否打开中继。"),
];

/// 数字中继与网关。
pub const DIGITAL_GATEWAYS: &[(&str, &str, &str)] = &[
  (
    "D-STAR",
    "DV 数字语音",
    "日本 ICOM 主导，支持互联网网关全球互联。",
  ),
  (
    "DMR",
    "TDMA 数字语音",
    "双时隙、色码与通话组，商业与业余广泛使用。",
  ),
  ("C4FM", "Fusion 数字语音", "日本八重洲主导，兼容模拟 FM。"),
  (
    "Hotspot",
    "MMDVM/Pi-Star 热点",
    "连接互联网网关的小型个人接入点，无需本地中继。",
  ),
];

/// 互联网网关。
pub const INTERNET_GATEWAYS: &[(&str, &str)] = &[
  ("EchoLink", "把电台经互联网互联，可用电脑/手机接入。"),
  ("IRLP", "互联网无线电连接项目，连接各地中继台。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn repeater_data_populated() {
    assert!(!REPEATER_CONCEPTS.is_empty());
    assert!(DIGITAL_GATEWAYS.len() >= 3);
    assert!(!INTERNET_GATEWAYS.is_empty());
  }
}
