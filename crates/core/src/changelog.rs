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
pub const CHANGELOG: &[(&str, &[&str])] = &[
  (
    "2026-10-02",
    &[
      "新增「开放 API」（/api/v1）：DXCC 实体与呼号解析、波段表、网格换算与距离方位、点对点传播预测，支持跨域、ETag 缓存与独立配额；配套文档页与 OpenAPI 3.1 文档",
      "新增「极光通信」专题：极光传播原理、Kp 指数、适用波段与操作要点",
      "新增「活动日历」：展会 / 火腿节与年度通联活动，可一键加入倒计时提醒",
      "新增「开源项目与 DIY 索引」：汇总社区开源软件与站内自制教程",
      "新增「SDR 在线接收站地图」：世界地图标注公开接收站，直达 WebSDR / KiwiSDR 生态",
      "新增「频率协调」专题：IARU 三区波段差异、协调层级、频率申请与干扰处理流程",
      "新增「呼号查询」页：本地解析 DXCC 实体 / CQ·ITU 分区 / 稀有度，可在线补全姓名与网格",
      "新增「设备评测与选购」：精选机型库、按用途推荐与多机参数对比",
      "新增「CQ / ITU 分区地图」：按分区着色世界地图，支持呼号定位与分区构成速查；呼号查询页内嵌分区小地图",
    ],
  ),
  (
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
  ),
];

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
