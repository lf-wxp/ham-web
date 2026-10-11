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

/// DMR 机理速查：时隙、色码与 Tier 分级。
pub const DMR_BASICS: &[(&str, &str)] = &[
  (
    "TDMA 时隙",
    "DMR 每 30ms 一帧分成两个时隙（Slot 1 / Slot 2），两部电台可在同一个 12.5kHz 信道上同时通话 —— 频分复用上再叠时分复用。",
  ),
  (
    "色码 Color Code",
    "0–15 共 16 个值，作用类似 PL 亚音：只有色码匹配的中继/台站才会解出你的信号，用于区分同一频率上的不同网络。",
  ),
  (
    "通话组与私有呼叫",
    "每个时隙上再按「通话组 Talkgroup」或单呼地址路由：同一中继的两组用户可以互不干扰地各自通话。",
  ),
  (
    "Tier I",
    "免执照直频：446MHz 免许可频段设备，无中继、不联网（与独立标准 dPMR 不是一回事）。",
  ),
  (
    "Tier II",
    "业余与商用最常用的中继/直频模式，TDMA 两时隙，可与 IP 骨干（BrandMeister 等）互连。",
  ),
  (
    "Tier III",
    "集群调度系统，带控制信道的 trunking 架构，商用为主，业余较少用。",
  ),
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
