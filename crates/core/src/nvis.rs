//! NVIS 近垂直入射天波：近距离盲区通信的天线技术。

/// 核心概念。
pub const NVIS_CONCEPTS: &[(&str, &str)] = &[
  ("NVIS", "Near Vertical Incidence Skywave，近垂直入射天波。"),
  (
    "原理",
    "天线向正上方辐射，经电离层近乎垂直反射，覆盖 0–500km 地波盲区。",
  ),
  ("适用频率", "通常 1.8–7.5 MHz，取决于当时电离层临界频率。"),
  ("典型波段", "白天 40m / 60m，夜间 80m / 160m。"),
  (
    "天线形态",
    "低架设的偶极、倒 V 或环形天线，架高约 1/10–1/4 波长。",
  ),
];

/// 应用与要点。
pub const NVIS_TIPS: &[&str] = &[
  "应急通信与山区、城市近距离通信的理想方案，无地波盲区。",
  "天线架设低（数米），主瓣朝上，牺牲远距离 DX 能力换取近距离覆盖。",
  "地面或金属网作反射面可增强向上辐射。",
  "根据实时临界频率选择波段，频率过高会穿透电离层失去反射。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nvis_data_populated() {
    assert!(NVIS_CONCEPTS.len() >= 4);
    assert!(!NVIS_TIPS.is_empty());
  }
}
