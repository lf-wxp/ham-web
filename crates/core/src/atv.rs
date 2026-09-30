//! 业余电视 ATV / DATV：快速扫描电视与数字电视。

/// 概念与类型。
pub const ATV_CONCEPTS: &[(&str, &str)] = &[
  (
    "ATV",
    "Amateur Television，业余电视，快速扫描传送运动图像。",
  ),
  (
    "SSTV",
    "Slow-Scan Television，慢扫描静止图像（话音带宽内）。",
  ),
  (
    "DATV",
    "Digital Amateur Television，基于 DVB-S / DVB-T 的数字电视。",
  ),
  ("带宽", "ATV 占用数 MHz 宽带，DATV 用压缩编码大幅降低码率。"),
  ("频段", "常用 UHF（如 430 / 1200MHz）及以上微波频段。"),
];

/// 操作要点。
pub const ATV_TIPS: &[&str] = &[
  "ATV 带宽大，需确认操作证允许的频段与发射类别。",
  "DATV 常用软件（如 DATV Express）配合 SDR 或专用调制器。",
  "电视信号对失真与干扰敏感，注意功放线性与馈线质量。",
  "国际空间站不定期开展 SSTV 活动，是入门图像接收的好机会。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn atv_data_populated() {
    assert!(ATV_CONCEPTS.len() >= 4);
    assert!(!ATV_TIPS.is_empty());
  }
}
