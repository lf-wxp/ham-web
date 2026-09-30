//! 国际组织与分区：主要业余无线电组织与 CQ / ITU 分区。

/// 主要组织。
pub const ORGS: &[(&str, &str, &str)] = &[
  (
    "ITU",
    "国际电信联盟（International Telecommunication Union）",
    "联合国下属机构，统一全球无线电频谱与呼号前缀分配。",
  ),
  (
    "IARU",
    "国际业余无线电联盟（International Amateur Radio Union）",
    "代表全球业余无线电爱好者，协调国际事务与频段。",
  ),
  (
    "ARRL",
    "美国业余无线电联盟（American Radio Relay League）",
    "美国最大业余组织，颁发 DXCC 等奖状、出版刊物。",
  ),
  (
    "JARL",
    "日本业余无线电联盟（Japan Amateur Radio League）",
    "日本业余组织，管理 JARL 奖状与活动。",
  ),
  (
    "RSGB",
    "英国无线电协会（Radio Society of Great Britain）",
    "英国业余组织，颁发 IOTA 奖状。",
  ),
  (
    "CRAC",
    "中国无线电协会业余无线电分会（Chinese Radio Amateurs Club）",
    "中国业余无线电全国性组织，负责题库与考试相关工作。",
  ),
];

/// 分区概念。
pub const ZONES: &[(&str, &str)] = &[
  (
    "CQ 分区",
    "全球划分为 40 个 CQ 分区，是 WAZ（Worked All Zones，全部 CQ 分区）奖状与竞赛的基础。",
  ),
  (
    "ITU 分区",
    "全球划分为 90 个 ITU 分区，用于呼号与频率协调。",
  ),
  (
    "中国所在分区",
    "中国大陆主要位于 CQ 23、24 区，ITU 42–44 区。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn organizations_data_populated() {
    assert!(ORGS.len() >= 5);
    assert!(!ZONES.is_empty());
    for (abbr, full, role) in ORGS {
      assert!(!abbr.is_empty());
      assert!(!full.is_empty());
      assert!(!role.is_empty());
    }
  }
}
