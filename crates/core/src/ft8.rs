//! FT8 / FT4 弱信号数字模式：WSJT-X 操作、标准频率与通联流程。

/// 核心概念。
pub const FT8_CONCEPTS: &[(&str, &str)] = &[
  (
    "FT8",
    concat!(
      "弱信号数字模式，15 秒周期，约 50Hz 带宽，8-FSK、音间隔 6.25Hz、77 比特消息。",
      "常见通联功率为 5–100W；几瓦即可完成远距离通联，",
      "mW 级只在极好传播 + 大天线 + 大批接收站的信标类场景才成立。",
    ),
  ),
  ("FT4", "FT8 的 7.5 秒快速版，适合比赛与快速交换。"),
  (
    "时间同步",
    "收发周期严格对齐 UTC，需用 NTP 同步时间；建议控制在 1 秒以内，超过约 2 秒基本解不出。",
  ),
  (
    "WSJT-X / JTDX",
    "最常用的 FT8/FT4 收发软件，连接电台声卡与 CAT 控制。",
  ),
];

/// 标准频率（使用 USB 模式）。
pub const FT8_FREQS: &[(&str, &str)] = &[
  ("1.840 MHz", "160m FT8"),
  ("3.573 MHz", "80m FT8"),
  ("7.074 MHz", "40m FT8"),
  ("10.136 MHz", "30m FT8"),
  ("14.074 MHz", "20m FT8"),
  ("18.100 MHz", "17m FT8"),
  ("21.074 MHz", "15m FT8"),
  ("24.915 MHz", "12m FT8"),
  ("28.074 MHz", "10m FT8"),
  ("50.313 MHz", "6m FT8"),
  ("144.174 MHz", "2m FT8"),
];

/// 操作要点。
pub const FT8_TIPS: &[&str] = &[
  "使用 USB 模式（即使频率在 10MHz 以下），软件通过音频实现收发。",
  "控制 ALC 不动作、不过载，保持信号干净。",
  "完整序列：CQ → 应答 → 信号报告 → RRR/73，由软件自动完成。",
  "日志自动记录，可导出 ADIF 与 LoTW 同步。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ft8_data_populated() {
    assert!(!FT8_CONCEPTS.is_empty());
    assert!(FT8_FREQS.len() >= 6);
    assert!(!FT8_TIPS.is_empty());
  }
}
