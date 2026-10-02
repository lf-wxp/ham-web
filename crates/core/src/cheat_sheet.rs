//! 高频考点速查手册：把分类体系里已整理的知识点文案（`categories::SUB_NOTES`）
//! 按一级分类组织，供「考点速查手册」页（`/cheat-sheet`）打印或考前冲刺浏览。

use std::collections::BTreeMap;

use crate::categories::{SUB_NOTES, TOP_CATEGORIES, sub_category};

/// 一条考点速查。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatSheetItem {
  /// 官方分类码 P（如 `3.3.2`）。
  pub code: &'static str,
  /// 二级分类（知识点）名。
  pub name: &'static str,
  /// 知识点速查文案。
  pub note: &'static str,
}

/// 一个一级分类下的考点分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatSheetSection {
  /// 一级分类 key。
  pub key: &'static str,
  /// 一级分类名。
  pub name: &'static str,
  /// 标识色。
  pub color: &'static str,
  /// 该分类下的考点。
  pub items: Vec<CheatSheetItem>,
}

/// 按一级分类组织的高频考点速查（只包含已整理知识点文案的考点，按定义顺序）。
#[must_use]
pub fn sections() -> Vec<CheatSheetSection> {
  let mut map: BTreeMap<&'static str, Vec<CheatSheetItem>> = BTreeMap::new();
  for &(code, note) in SUB_NOTES {
    let Some(sub) = sub_category(code) else {
      continue;
    };
    map.entry(sub.top).or_default().push(CheatSheetItem {
      code,
      name: sub.name,
      note,
    });
  }
  TOP_CATEGORIES
    .iter()
    .filter_map(|top| {
      let items = map.get(top.key)?;
      Some(CheatSheetSection {
        key: top.key,
        name: top.name,
        color: top.color,
        items: items.clone(),
      })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sections_group_by_top_and_keep_all_notes() {
    let secs = sections();
    assert_eq!(secs.len(), 10, "应覆盖全部 10 个一级分类");
    for s in &secs {
      assert!(!s.items.is_empty(), "{} 应有考点", s.key);
      for it in &s.items {
        assert!(!it.name.is_empty() && !it.note.is_empty());
        assert_eq!(sub_category(it.code).map(|c| c.top), Some(s.key));
      }
    }
    // 考点总数应与 SUB_NOTES 中能映射到二级分类的条目一致。
    let total: usize = secs.iter().map(|s| s.items.len()).sum();
    let mapped = SUB_NOTES
      .iter()
      .filter(|(code, _)| sub_category(code).is_some())
      .count();
    assert_eq!(total, mapped);
  }
}
