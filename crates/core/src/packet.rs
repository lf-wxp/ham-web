//! Packet Radio 分组无线电与 Winlink 邮件网关。

/// 核心概念。
pub const PACKET_CONCEPTS: &[(&str, &str)] = &[
  ("Packet", "把数据切成带地址的数据包，用 AX.25 协议传输。"),
  ("AX.25", "业余分组通信的链路层协议，含源/目的呼号与校验。"),
  (
    "AX.25 帧",
    "以 0x7E 标志定界，含地址字段（呼号 + SSID，含扩展位）、控制与 PID 字段、信息字段及 CRC-16 校验；链路用 NRZI 编码与比特填充。",
  ),
  (
    "KISS",
    "Keep It Simple Stupid，TNC 与主机间的串行封装：0xC0 定界、0xDB 转义，使软件可替代硬件 TNC。",
  ),
  ("TNC", "终端节点控制器，在数据流与电台音频之间相互转换。"),
  ("Digipeater", "数字中继，按呼号路径逐跳转发分组。"),
  ("速率", "VHF/UHF 常用 1200 / 9600 baud，HF 常用 300 baud。"),
  (
    "Bell 202（1200 baud）",
    "VHF 主流：AFSK，mark 1200 Hz、space 2200 Hz，1200 baud、1 个起始位 + 8 数据位 + 1 个停止位。",
  ),
  (
    "G3RUH（9600 baud）",
    "直接 FSK 调制（不经音频 AFSK），需电台有平坦的 9600 接口与足够带宽，不能用普通话音麦克风口。",
  ),
];

/// 典型应用。
pub const PACKET_APPS: &[(&str, &str)] = &[
  ("APRS", "基于 Packet 的位置、气象与短消息自动广播。"),
  ("Packet BBS", "业余分组公告板，可留言与收发文件。"),
  (
    "Winlink",
    "通过 HF/VHF 电台收发电子邮件的全球网关，应急通信刚需。",
  ),
];

/// 操作要点。
pub const PACKET_TIPS: &[&str] = &[
  "现代 TNC 可用声卡 + 软件（如 Direwolf）替代专用硬件。",
  "Winlink 用 Winlink Express 接入，支持 Pactor / ARDOP / VARA 等协议。",
  "HF Packet 速度慢但覆盖远，适合长距离与应急场景。",
  "收发前确认频率占用情况，分组信号勿占用话音通联频率。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn packet_data_populated() {
    assert!(PACKET_CONCEPTS.len() >= 4);
    assert!(PACKET_APPS.len() >= 3);
    assert!(!PACKET_TIPS.is_empty());
  }
}
