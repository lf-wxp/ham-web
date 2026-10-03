//! Winlink 无线邮件系统：通过业余无线电收发电子邮件。

/// 核心概念。
pub const WINLINK_CONCEPTS: &[(&str, &str)] = &[
  (
    "Winlink",
    "全球业余无线电邮件系统，通过 HF / VHF 把邮件经 RMS 网关接入互联网。",
  ),
  (
    "RMS 网关",
    "Radio Message Server，接收无线电信号并转发邮件的网关台站。",
  ),
  (
    "声卡调制",
    "VARA / ARDOP 等软件调制解调，把数据编码成音频经电台收发。",
  ),
  (
    "PACTOR",
    "硬件 TNC 调制解调协议，HF 邮件传输能力强，但需专用设备。",
  ),
];

/// 常用调制方式。
pub const WINLINK_MODES: &[(&str, &str, &str)] = &[
  (
    "VARA HF",
    "声卡（免费）",
    "自适应速率 ARQ，Winlink 最常用的 HF 调制。",
  ),
  (
    "VARA FM",
    "声卡（免费）",
    "VHF / UHF 高速数据，约 9.6 kbps 量级。",
  ),
  (
    "ARDOP",
    "声卡（开源）",
    "开源 ARQ 调制，兼容 Winlink 与其它邮件系统。",
  ),
  (
    "PACTOR 4",
    "硬件 TNC（付费）",
    "HF 最强邮件调制，但需 SCS 调制解调器。",
  ),
];

/// 应急用法与要点。
pub const WINLINK_TIPS: &[&str] = &[
  "灾害时手机与互联网中断，可用 Winlink 通过 HF 远程收发邮件报平安。",
  "Winlink 需要「电台 + 电脑声卡（或 TNC）+ Winlink Express 客户端」。",
  "先找到能通联的 RMS 网关频率与时间表，再尝试连接。",
  "配合 EmComm 演练，提前注册 Winlink 账户并熟悉客户端。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn winlink_data_populated() {
    assert!(WINLINK_CONCEPTS.len() >= 3);
    assert!(WINLINK_MODES.len() >= 3);
    assert!(!WINLINK_TIPS.is_empty());
  }
}
