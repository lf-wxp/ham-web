//! DXCC 稀有度榜单：Most Wanted 与追台策略。

/// 稀有度概念。
pub const WANTED_CONCEPTS: &[(&str, &str)] = &[
  (
    "Most Wanted",
    "按全球需求排行的稀有 DXCC 实体榜单，反映稀有程度。",
  ),
  (
    "稀有度来源",
    "由 Club Log 等平台根据全球通联日志统计活跃度。",
  ),
  ("实体数量", "DXCC 实体约 340 个，其中部分常年无人设台。"),
  ("追台价值", "稀有实体一旦有远征队出现，会吸引全球 pileup。"),
];

/// 追台策略。
pub const WANTED_TIPS: &[&str] = &[
  "把最想要的实体加入守候列表，关注其远征动态。",
  "稀有台出现时按分区耐心等待，不要反复无序呼叫。",
  "利用传播预测选择最佳时间与波段冲击稀有实体。",
  "通联成功后及时确认（LoTW / QSL），避免错失奖状进度。",
];

/// 内置的知名稀有 DXCC 实体清单（呼号前缀, 实体名, 稀有度），用于本地通联进度追踪。
pub const WANTED_ENTITIES: &[(&str, &str, &str)] = &[
  ("P5", "朝鲜", "极稀有"),
  ("3Y/B", "布韦岛", "极稀有"),
  ("FT5/W", "克罗泽岛", "极稀有"),
  ("KH1", "贝克岛 / 豪兰岛", "极稀有"),
  ("7O", "也门", "极稀有"),
  ("BV9P", "东沙群岛", "稀有"),
  ("VK0H", "赫德岛", "稀有"),
  ("FT5/X", "凯尔盖朗岛", "稀有"),
  ("ZS8", "爱德华王子 / 马里昂岛", "稀有"),
  ("T33", "巴纳巴岛", "稀有"),
  ("9N", "尼泊尔", "稀有"),
  ("XZ", "缅甸", "稀有"),
  ("3C", "赤道几内亚", "稀有"),
  ("FT5/Z", "阿姆斯特丹岛", "稀有"),
  ("3Y/P", "彼得一世岛", "稀有"),
  ("VP8S", "南桑威奇群岛", "稀有"),
  ("4W", "东帝汶", "较稀有"),
  ("E4", "巴勒斯坦", "较稀有"),
  ("A5", "不丹", "较稀有"),
  ("9U", "布隆迪", "较稀有"),
];

/// 判断呼号是否命中某个稀有 DXCC 实体（按前缀最长匹配），返回命中的前缀。
#[must_use]
pub fn wanted_prefix(callsign: &str) -> Option<&'static str> {
  let cs = callsign.trim().to_ascii_uppercase();
  if cs.is_empty() {
    return None;
  }
  let mut best: Option<(usize, &'static str)> = None;
  for &(prefix, _, _) in WANTED_ENTITIES {
    if cs.starts_with(prefix) && best.is_none_or(|(len, _)| prefix.len() > len) {
      best = Some((prefix.len(), prefix));
    }
  }
  best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn most_wanted_data_populated() {
    assert!(WANTED_CONCEPTS.len() >= 4);
    assert!(!WANTED_TIPS.is_empty());
  }
}
