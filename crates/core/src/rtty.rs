//! RTTY 与 PSK31：传统数据模式的操作与常用频率。

/// 模式要点。
pub const RTTY_MODES: &[(&str, &str, &str)] = &[
  ("RTTY", "无线电传", "45.45 波特、170Hz 频移，竞赛主力模式。"),
  ("PSK31", "相移键控", "31.25 波特窄带，适合弱信号键盘聊天。"),
  ("PSK63", "高速相移键控", "62.5 波特，竞赛时速度翻倍。"),
  (
    "JT65",
    "弱信号",
    "1 分钟一个周期，用于 EME 与小功率远距离。",
  ),
  (
    "JS8Call",
    "弱信号聊天",
    "基于 FT8 信号结构，支持实时文字对话。",
  ),
];

/// 常用频率。
pub const RTTY_FREQS: &[(&str, &str)] = &[
  ("3.580 MHz", "80m RTTY。"),
  ("7.043 MHz", "40m RTTY（中国）。"),
  ("14.080 MHz", "20m RTTY。"),
  ("21.080 MHz", "15m RTTY。"),
  ("28.080 MHz", "10m RTTY。"),
];

/// 操作要点。
pub const RTTY_TIPS: &[&str] = &[
  "竞赛中 RTTY 常用「CQ TEST」加呼号，交换序号。",
  "控制音频驱动电平，避免过载导致信号失真。",
  "RTTY 用 FSK 或 AFSK，注意与收发信机选通配合。",
  "PSK31 功率无需太大，5–30W 足以稳定通联。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rtty_data_populated() {
    assert!(RTTY_MODES.len() >= 4);
    assert!(RTTY_FREQS.len() >= 4);
    assert!(!RTTY_TIPS.is_empty());
  }
}
