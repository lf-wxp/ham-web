//! 更新日志：编译进前端用于「本次更新」提示，构建时也会导出为 `/changelog.json`，
//! 让旧版本页面在发现新版本时就能展示新版本带来了什么。
//!
//! 发布新版本时在 [`CHANGELOG`] 最前面追加一条（日期新的在前）。

use serde::{Deserialize, Serialize};

/// 一次发布的更新内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
  /// 发布日期 `YYYY-MM-DD`，同时作为版本标识（按字符串比较先后）。
  pub date: String,
  pub items: Vec<String>,
}

/// 更新日志（日期新的在前）。
pub const CHANGELOG: &[(&str, &[&str])] = &[(
  "2026-10-01",
  &[
    "新版本与题库更新会在页面底部提示，并列出更新内容；题库更新后告诉你新增、修改了哪些题",
    "知识卡片复习：Q 简语、术语、字母解释法、莫尔斯字符按自适应间隔复习，首页显示今日待复习",
    "听题模式：自动朗读题目、选项与答案，适合通勤路上刷题",
    "薄弱项组卷：按分类正确率与错题加权出卷，交卷后对比各分类变化",
    "错题本改为自适应复习间隔，常错的题需要连续答对更多次才移出",
    "手机上左右滑动切题；闪卡可滑动自评",
    "通联日志：电台 CAT 联动自动填频率 / 模式，QSL 标签打印",
    "莫尔斯电码页新增麦克风 CW 解码",
  ],
)];

/// 当前版本（最新一条更新日志的日期）。
#[must_use]
pub fn current() -> &'static str {
  CHANGELOG.first().map_or("", |(date, _)| date)
}

/// 编译进来的更新日志。
#[must_use]
pub fn releases() -> Vec<Release> {
  CHANGELOG
    .iter()
    .map(|(date, items)| Release {
      date: (*date).to_owned(),
      items: items.iter().map(|s| (*s).to_owned()).collect(),
    })
    .collect()
}

/// 晚于 `since` 的更新条目（按日期新到旧），最多 `limit` 条。
#[must_use]
pub fn items_since(releases: &[Release], since: &str, limit: usize) -> Vec<String> {
  let mut list: Vec<&Release> = releases
    .iter()
    .filter(|r| r.date.as_str() > since)
    .collect();
  list.sort_by(|a, b| b.date.cmp(&a.date));
  list
    .into_iter()
    .flat_map(|r| r.items.iter().cloned())
    .take(limit)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn changelog_is_sorted_and_dated() {
    for w in CHANGELOG.windows(2) {
      assert!(
        w[0].0 > w[1].0,
        "更新日志需按日期从新到旧：{} / {}",
        w[0].0,
        w[1].0
      );
    }
    for (date, items) in CHANGELOG {
      assert_eq!(date.len(), 10, "日期格式应为 YYYY-MM-DD：{date}");
      assert!(!items.is_empty());
    }
    assert_eq!(current(), CHANGELOG[0].0);
  }

  #[test]
  fn picks_newer_items() {
    let list = vec![
      Release {
        date: "2026-01-01".into(),
        items: vec!["旧".into()],
      },
      Release {
        date: "2026-03-01".into(),
        items: vec!["新 1".into(), "新 2".into()],
      },
      Release {
        date: "2026-02-01".into(),
        items: vec!["中".into()],
      },
    ];
    assert_eq!(
      items_since(&list, "2026-01-01", 10),
      vec!["新 1", "新 2", "中"]
    );
    assert_eq!(items_since(&list, "2026-01-01", 2), vec!["新 1", "新 2"]);
    assert!(items_since(&list, "2026-03-01", 10).is_empty());
  }
}
