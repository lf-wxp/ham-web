//! 户外便携操作：SOTA、POTA 等热门项目与便携装备。

/// 主要项目。
pub const PORTABLE_PROGRAMS: &[(&str, &str, &str)] = &[
  (
    "SOTA",
    "Summits On The Air，山峰通联",
    "在指定山峰登顶设台，激活者与追逐者均可积分。",
  ),
  (
    "POTA",
    "Parks On The Air，公园通联",
    "在指定公园内设台，与 SOTA 类似但更易参与。",
  ),
  (
    "WWFF",
    "World Wide Flora & Fauna，全球动植物保护区",
    "在保护区设台，国际版 POTA。",
  ),
];

/// 便携要点。
pub const PORTABLE_TIPS: &[&str] = &[
  "装备轻量化：便携电台 + 便携天线 + 电池（磷酸铁锂）。",
  "常用频段：20m/40m SSB 或 CW，配合 FT8/FT4 快速积累通联。",
  "提前发布激活计划（SOTAwatch 等），让追逐者守候。",
  "户外注意安全：防雷、防雨、携带充足电源与饮水。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn portable_data_populated() {
    assert!(PORTABLE_PROGRAMS.len() >= 2);
    assert!(!PORTABLE_TIPS.is_empty());
  }
}
