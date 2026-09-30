//! 传播预测与最佳工作频率：MUF、LUF、OWF 与波段选择。

/// 核心概念。
pub const MUF_CONCEPTS: &[(&str, &str)] = &[
  (
    "MUF 最高可用频率",
    "电离层能反射回地面的最高频率，高于它信号穿透电离层。",
  ),
  (
    "LUF 最低可用频率",
    "能被接收的最低频率，低于它信号被吸收或噪声淹没。",
  ),
  ("OWF 最佳工作频率", "约为 MUF 的 85%，最可靠的通信频率。"),
  (
    "VOACAP",
    "业界常用的传播预测软件，输入两端位置与时间预测可用波段。",
  ),
];

/// 波段选择建议。
pub const BAND_CHOICE: &[(&str, &str)] = &[
  ("白天 · 高太阳活动", "15m、10m 等高波段开放良好。"),
  ("夜间", "40m、80m 等低波段更适合远距离。"),
  ("日出日落", "各波段的黄金时间，传播变化快。"),
  ("太阳活动低年", "低波段（160m/80m/40m）更可靠，高波段受限。"),
];

/// 估算 F2 层临界频率 foF2（MHz）：基于太阳通量 SFI 的经验近似。
#[must_use]
pub fn estimate_fof2(sfi: f64) -> f64 {
  if sfi <= 0.0 {
    return 0.0;
  }
  0.5 + 0.9 * sfi.sqrt()
}

/// 估算单跳 F2 最高可用频率 MUF（MHz）。
#[must_use]
pub fn estimate_muf(sfi: f64) -> f64 {
  estimate_fof2(sfi) * 3.0
}

/// 最佳工作频率 OWF（MHz），约为 MUF 的 85%。
#[must_use]
pub fn estimate_owf(sfi: f64) -> f64 {
  estimate_muf(sfi) * 0.85
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn muf_data_populated() {
    assert!(!MUF_CONCEPTS.is_empty());
    assert!(!BAND_CHOICE.is_empty());
  }

  #[test]
  fn fof2_estimates_in_reasonable_range() {
    assert_eq!(estimate_fof2(0.0), 0.0);
    let f = estimate_fof2(150.0);
    assert!((f - 11.5).abs() < 2.0, "foF2 {f}");
    let m = estimate_muf(150.0);
    assert!(m > f, "muf {m} > foF2 {f}");
    let o = estimate_owf(150.0);
    assert!(o > 0.0 && o < m);
  }
}
