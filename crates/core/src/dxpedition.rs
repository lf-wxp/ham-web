//! DX 远征（DXpedition）：稀有 DXCC 实体远征队的组织与参与。

/// 核心概念。
pub const DXPED_CONCEPTS: &[(&str, &str)] = &[
  (
    "DXpedition",
    "组织团队前往稀有或无人 DXCC 实体临时设台，满足全球通联需求。",
  ),
  (
    "稀有度",
    "按每年活跃台数量划分 Most Wanted，越稀有追台越热烈。",
  ),
  (
    "团队配置",
    "多名操作员、多台设备、多个波段/模式同时工作，昼夜轮班。",
  ),
  ("经费来源", "基金会赞助、个人募捐，以及 OQRS 通联确认费用。"),
  (
    "日志上传",
    "远征期间或结束上传 Club Log 与 LoTW，供全球核对确认。",
  ),
];

/// 追远征台要点。
pub const DXPED_TIPS: &[&str] = &[
  "提前在 DX Cluster 与公告板了解远征时间与频率计划。",
  "严格遵守 split 规则，只在其指定收听频率呼叫。",
  "远征台常按分区、前缀或大洲点名，耐心等待轮到你。",
  "通过 OQRS 直接请求 QSL 卡，或等待 LoTW 上传确认。",
  "不制造干扰：重复呼叫、在错误频率发射只会拖慢全场。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dxpedition_data_populated() {
    assert!(DXPED_CONCEPTS.len() >= 4);
    assert!(!DXPED_TIPS.is_empty());
  }
}
