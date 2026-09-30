//! 远程电台 Remote Station：远程操作架构与搭建。

/// 核心概念。
pub const REMOTE_CONCEPTS: &[(&str, &str)] = &[
  (
    "远程操作",
    "通过互联网从异地控制电台，用于在位置受限时使用高性能台站。",
  ),
  (
    "控制协议",
    "CAT 串口命令经网络转发，配合音频流（VoIP）实现远程收发。",
  ),
  (
    "常见方案",
    "RemoteRig、FlexRadio SmartSDR、开源的 OpenWebRX/RemoteTx 等。",
  ),
];

/// 搭建要点。
pub const REMOTE_TIPS: &[&str] = &[
  "远程端需稳定网络与备份电源，避免失联后电台无法关停。",
  "做好安全：鉴权、加密、限权，防止未授权使用。",
  "遵守执照规定，远程台站仍由持证操作员负责。",
  "音频延迟与网络抖动会影响操作，优先有线/低延迟网络。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn remote_data_populated() {
    assert!(!REMOTE_CONCEPTS.is_empty());
    assert!(!REMOTE_TIPS.is_empty());
  }
}
