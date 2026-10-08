//! 本地数据占用的展示整形：把「某个 key 占多少」按界面分组汇总。
//!
//! 归并（同一分组的 key 求和）与排序（按占用降序）都是纯逻辑，因此放在 core；
//! 分组标签由调用方给 —— core 不产界面文案。

use std::collections::BTreeMap;

/// 把 `(key, 占用)` 按 `group_of` 归并求和，返回按占用降序的 `(分组, 总占用)`。
///
/// 占用相同时按分组名升序：同一份数据每次渲染的先后必须一致，否则备份页那份占用列表
/// 会在每次重算时莫名换位。占用的单位由调用方决定（本工具是 localStorage 的 UTF-16 单元数）。
#[must_use]
pub fn sum_by_group<'a, G: Ord>(
  items: impl IntoIterator<Item = (&'a str, usize)>,
  group_of: impl Fn(&str) -> G,
) -> Vec<(G, usize)> {
  let mut map: BTreeMap<G, usize> = BTreeMap::new();
  for (key, units) in items {
    *map.entry(group_of(key)).or_default() += units;
  }
  let mut out: Vec<(G, usize)> = map.into_iter().collect();
  out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn groups_are_summed_and_sorted_by_usage() {
    let items = [
      ("logbook", 300),
      ("mistake-book", 100),
      ("station-info", 50),
      ("theme", 1),
    ];
    let group = |key: &str| match key {
      "logbook" | "station-info" => "log",
      "mistake-book" => "mistakes",
      _ => "other",
    };
    assert_eq!(
      sum_by_group(items, group),
      vec![("log", 350), ("mistakes", 100), ("other", 1)]
    );
  }

  #[test]
  fn ties_fall_back_to_the_group_order() {
    // 占用一样多时按分组名升序：不这样的话先后随 `BTreeMap` 之外的实现细节变化。
    let group = |key: &str| key.to_owned();
    assert_eq!(
      sum_by_group([("b", 5), ("a", 5)], group),
      vec![("a".to_owned(), 5), ("b".to_owned(), 5)]
    );
  }

  #[test]
  fn an_empty_usage_list_yields_nothing() {
    assert!(sum_by_group([], |k: &str| k.to_owned()).is_empty());
  }
}
