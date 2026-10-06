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
  (
    "ARQ 与自适应速率",
    "ARQ（自动重传请求）把数据分块发送，收方校验出错即请求重发；自适应速率会随信道质量升降速率，故弱传播时仍能连通、好传播时提速。",
  ),
  (
    "FEC 前向纠错",
    "发送端加入冗余，收端可自行纠错而无需重传，适合单向广播；实际系统常把 ARQ 与 FEC 结合使用。",
  ),
];

/// 选频说明：Winlink 没有全球统一的呼叫频率。
pub const WINLINK_FREQ_NOTES: &[&str] = &[
  "Winlink 没有全球单一呼叫频率，应按目标 RMS 网关公布的频点选频（软件内置网关列表会按波段与时段给出可用频点）。",
  "HF 上通常在各波段的数据 / 自动子段内工作（避开 CW 与话音段，也避开 FT8 等拥挤频点）。",
  "VHF/UHF 走本地 RMS 网关的固定频点，与 APRS 频率不同，不要混用。",
  "先听再叫：确认频点空闲且能收到网关信标再发起连接。",
];

/// 常用调制方式。
pub const WINLINK_MODES: &[(&str, &str, &str)] = &[
  (
    "VARA HF",
    "声卡（付费注册，有试用版）",
    "自适应速率 ARQ，Winlink 最常用的 HF 调制。",
  ),
  (
    "VARA FM",
    "声卡（付费注册，有试用版）",
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
  (
    "PACTOR 1–3",
    "硬件 TNC（向下兼容）",
    "PACTOR 早期版本，被 PACTOR 4 向下兼容；部分网关仍在使用。",
  ),
  (
    "Packet / Telnet",
    "声卡 / TNC / 网络",
    "部分本地 RMS 以 VHF/UHF 分组（Packet）或 Telnet 方式接入，用于近距离或应急。",
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
