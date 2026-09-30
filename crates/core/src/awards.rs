//! DX 奖状体系：业余无线电主要国际奖状的简称、全称与基本规则。

/// (简称, 全称, 颁发机构, 基本规则)
pub const AWARDS: &[(&str, &str, &str, &str)] = &[
  (
    "DXCC",
    "DX 世纪俱乐部（DX Century Club）",
    "ARRL",
    "与 100 个以上 DXCC 实体通联并确认，按波段/模式细分。",
  ),
  (
    "WAZ",
    "全部 CQ 分区（Worked All Zones）",
    "CQ 杂志",
    "通联全部 40 个 CQ 分区。",
  ),
  (
    "WAS",
    "全部美国州（Worked All States）",
    "ARRL",
    "通联美国全部 50 个州。",
  ),
  (
    "WAC",
    "全部大洲（Worked All Continents）",
    "IARU",
    "通联全部 6 大洲。",
  ),
  (
    "IOTA",
    "空中岛屿（Islands On The Air）",
    "RSGB",
    "通联规定数量的海岛组。",
  ),
  (
    "VUCC",
    "VHF/UHF 世纪俱乐部（VHF/UHF Century Club）",
    "ARRL",
    "VHF/UHF 通联 100 个网格定位方块。",
  ),
  (
    "WPX",
    "全部呼号前缀（Worked All Prefixes）",
    "CQ 杂志",
    "通联规定数量的不同呼号前缀。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn awards_data_populated() {
    assert!(AWARDS.len() >= 6);
    for (abbr, full, org, rule) in AWARDS {
      assert!(!abbr.is_empty());
      assert!(!full.is_empty());
      assert!(!org.is_empty());
      assert!(!rule.is_empty());
    }
  }
}
