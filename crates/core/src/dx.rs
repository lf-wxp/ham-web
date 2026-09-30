//! DX 通联技巧：分裂操作、DX 窗口、pileup 礼仪与稀有台追逐。

/// 核心概念。
pub const DX_CONCEPTS: &[(&str, &str)] = &[
  (
    "分裂操作 Split",
    "DX 电台在 A 频率收听、B 频率发射，避免互相干扰，是追 DX 的基本功。",
  ),
  (
    "收听间隙 Listening up",
    "DX 台指定在发射频率上方 1–10 kHz 收听，需把发射频率设到其收听频率。",
  ),
  (
    "DX 窗口",
    "各波段专供 DX 台发射、本地台守听的约定频段（如 20m 14.190–14.200）。",
  ),
  (
    "Pileup",
    "大量电台同时呼叫稀有台形成的堆积，需遵守礼仪、耐心守听。",
  ),
];

/// 追台要点。
pub const DX_TIPS: &[&str] = &[
  "先守听，弄清 DX 台的收听频率（split 方向）再呼叫，不要在其发射频率上呼叫。",
  "只报自己呼号，简洁重复 1–2 次即可，避免冗长。",
  "被点到才发射，未点到勿反复呼叫，避免制造干扰。",
  "稀有台按分区/前缀点名时，耐心等待轮到你的分区。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dx_data_populated() {
    assert!(!DX_CONCEPTS.is_empty());
    assert!(!DX_TIPS.is_empty());
  }
}
