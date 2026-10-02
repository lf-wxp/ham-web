//! 收藏分组：把收藏题目归入自定义分组，便于按主题组织与复习。
//!
//! 收藏本身仍以扁平集合（`stable_id`）存储，分组是其上的标签子集，
//! 与旧版 `bookmarks` 数据完全兼容。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// 一个收藏分组。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkGroup {
  pub name: String,
  /// 该分组下的题目 `stable_id` 集合（有序）。
  #[serde(default)]
  pub ids: Vec<String>,
}

/// 全部收藏分组。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkGroups {
  #[serde(default)]
  pub groups: Vec<BookmarkGroup>,
}

impl BookmarkGroups {
  /// 新建分组；空名或重名返回 `false`。
  pub fn create(&mut self, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || self.groups.iter().any(|g| g.name == name) {
      return false;
    }
    self.groups.push(BookmarkGroup {
      name: name.to_owned(),
      ids: Vec::new(),
    });
    true
  }

  /// 重命名分组；空名或与其他组重名返回 `false`。
  pub fn rename(&mut self, old: &str, new: &str) -> bool {
    let new = new.trim();
    if new.is_empty() || self.groups.iter().any(|g| g.name == new && g.name != old) {
      return false;
    }
    self
      .groups
      .iter_mut()
      .find(|g| g.name == old)
      .is_some_and(|g| {
        g.name = new.to_owned();
        true
      })
  }

  /// 删除分组（不删除题目本身）。
  pub fn delete(&mut self, name: &str) {
    self.groups.retain(|g| g.name != name);
  }

  /// 把题目加入分组；分组不存在或已在组内返回 `false`。
  pub fn add(&mut self, name: &str, id: &str) -> bool {
    let Some(g) = self.groups.iter_mut().find(|g| g.name == name) else {
      return false;
    };
    if g.ids.iter().any(|x| x == id) {
      return false;
    }
    g.ids.push(id.to_owned());
    true
  }

  /// 把题目移出分组。
  pub fn remove(&mut self, name: &str, id: &str) -> bool {
    let Some(g) = self.groups.iter_mut().find(|g| g.name == name) else {
      return false;
    };
    let before = g.ids.len();
    g.ids.retain(|x| x != id);
    g.ids.len() != before
  }

  /// 题目所属的所有分组名（按分组定义顺序）。
  #[must_use]
  pub fn groups_of(&self, id: &str) -> Vec<String> {
    self
      .groups
      .iter()
      .filter(|g| g.ids.iter().any(|x| x == id))
      .map(|g| g.name.clone())
      .collect()
  }

  /// 清除所有分组中不在 `valid_ids` 里的题目（收藏被取消后清理残留）。
  pub fn prune(&mut self, valid_ids: &HashSet<String>) {
    for g in &mut self.groups {
      g.ids.retain(|x| valid_ids.contains(x));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_rename_delete() {
    let mut g = BookmarkGroups::default();
    assert!(g.create("法规"));
    assert!(!g.create("法规")); // 重名
    assert!(!g.create("  ")); // 空名
    assert!(g.rename("法规", "无线电法规"));
    assert!(!g.rename("无线电法规", "")); // 空名
    assert!(!g.rename("不存在", "x"));
    g.delete("无线电法规");
    assert!(g.groups.is_empty());
  }

  #[test]
  fn add_remove_and_groups_of() {
    let mut g = BookmarkGroups::default();
    g.create("易混淆");
    g.create("天线");
    assert!(g.add("易混淆", "a"));
    assert!(!g.add("易混淆", "a")); // 重复
    assert!(!g.add("不存在", "a"));
    g.add("天线", "a");
    assert_eq!(
      g.groups_of("a"),
      vec!["易混淆".to_owned(), "天线".to_owned()]
    );
    assert!(g.remove("易混淆", "a"));
    assert_eq!(g.groups_of("a"), vec!["天线".to_owned()]);
  }

  #[test]
  fn prune_removes_stale_ids() {
    let mut g = BookmarkGroups::default();
    g.create("g");
    g.add("g", "a");
    g.add("g", "b");
    let valid = HashSet::from(["a".to_owned()]);
    g.prune(&valid);
    assert_eq!(g.groups[0].ids, vec!["a".to_owned()]);
  }
}
