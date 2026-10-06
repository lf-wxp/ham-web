//! 数字语音组网：D-STAR、DMR 与 C4FM 的联网架构。

/// 主要网络。
pub const DV_NETWORKS: &[(&str, &str, &str)] = &[
  (
    "D-STAR",
    "Icom",
    "通过网关与反射器（Reflector）实现全球联网。",
  ),
  (
    "DMR",
    "多厂商",
    "BrandMeister、DMR-MARC 等网络，按时隙与通话组路由。",
  ),
  (
    "System Fusion",
    "Yaesu",
    "WIRES-X 网络，支持房间与节点互联。",
  ),
  ("FreeDMR", "开源", "开源 DMR 网络，可与多协议桥接。"),
];

/// 接入要点。
pub const DV_NETWORK_TIPS: &[&str] = &[
  "DMR 需配置色码、时隙与通话组，匹配当地中继设置。",
  "D-STAR 反射器/网关的模块字母（A / B / C）通常对应频段：A=23cm、B=70cm、C=2m。",
  "WIRES-X 按房间号加入，支持跨洲语音与数据传输。",
  "遵守当地中继的使用规范，避免长时间占用通话组。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dv_network_data_populated() {
    assert!(DV_NETWORKS.len() >= 4);
    assert!(!DV_NETWORK_TIPS.is_empty());
  }
}
