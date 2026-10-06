//! 收发信机原理与选购：接收机关键指标、超外差架构与选购要点。

// 接收机指标与噪声概念的**单一数据源**在 `receiver` / `noise` 模块，这里只做再导出。
// 过去两个模块各写一份且口径不一致（例如动态范围只提 P1dB/IP3 而漏掉噪底这一半），
// 与 `knowledge_consistency.rs` 的建库初衷相悖。
pub use crate::noise::NOISE_CONCEPTS;
pub use crate::receiver::{NOISE_BASICS, RECEIVER_METRICS};

/// 超外差与发射机概念。
pub const TRANSCEIVER_CONCEPTS: &[(&str, &str)] = &[
  (
    "超外差",
    "把接收信号与本振混频，变频到固定中频（IF）再处理，选择性高。",
  ),
  (
    "镜像频率",
    "与本振相差一个中频的对称频率，抑制不足会产生假响应。",
  ),
  (
    "末级功放",
    "发射机的功率放大级，决定输出功率，需良好匹配天线。",
  ),
  (
    "驻波保护",
    "天线失配时自动降低功率，保护功放不被反射功率损坏。",
  ),
];

/// 选购要点。
pub const BUYING_TIPS: &[&str] = &[
  "按用途选择：本地通联选手台/车台（VHF/UHF），远距离 DX 选短波电台（HF）。",
  "关注接收指标：灵敏度、选择性、动态范围比发射功率更重要。",
  "考虑扩展：内置天调、数字模式接口（CAT/USB）、SDR 频谱显示。",
  "预算内优先保证天线与馈线质量，天线系统往往比电台本身更关键。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn transceiver_data_populated() {
    assert!(RECEIVER_METRICS.len() >= 4);
    assert!(!TRANSCEIVER_CONCEPTS.is_empty());
    assert!(!BUYING_TIPS.is_empty());
    assert!(!NOISE_CONCEPTS.is_empty());
  }
}
