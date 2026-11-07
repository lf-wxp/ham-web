//! 考试日程的查法：本站不发布排期，只整理「去哪里查」与「查到之后做什么」。
//!
//! 排期由各省无线电管理机构在报名公告里发布，全国没有统一的接口或页面，逐个抓取也不现实
//! （各地入口不同、公告还会撤下）。因此这里只维护**渠道与核对要点**，不维护日期本身 ——
//! 落地页面上是一份永远不过期的指引，而不是一份注定过期的日历。
//!
//! 文案全部是界面词典的 key（`data/i18n/*/learning.json`），这里只保留顺序与外链：
//! 改措辞不必碰代码，译文按语言各自维护。

/// 一条查询渠道。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
  /// 渠道名（词典 key）。
  pub name: &'static str,
  /// 这个渠道能查到什么（词典 key）。
  pub what: &'static str,
  /// 在渠道里的具体入口（词典 key）：没有统一网址的渠道，靠这段文字告诉用户怎么找。
  pub entry: &'static str,
  /// 官网地址；`None` 表示没有统一入口。
  pub url: Option<&'static str>,
}

/// 查询渠道，按「先看哪里」排序：全国性机构 → 省级机构 → 考点 / 协会。
pub const CHANNELS: &[Channel] = &[
  Channel {
    name: "learning.exam-schedule-channel-crac",
    what: "learning.exam-schedule-crac-what",
    entry: "learning.exam-schedule-crac-entry",
    url: Some("https://www.crac.org.cn/"),
  },
  Channel {
    name: "learning.exam-schedule-channel-province",
    what: "learning.exam-schedule-province-what",
    entry: "learning.exam-schedule-province-entry",
    url: None,
  },
  Channel {
    name: "learning.exam-schedule-channel-venue",
    what: "learning.exam-schedule-venue-what",
    entry: "learning.exam-schedule-venue-entry",
    url: None,
  },
];

/// 看到公告后要核对的事项 `(名称, 说明)`，都是词典 key。
///
/// 这一段比渠道清单更重要：公告里真正容易看漏的是类别门槛、截止时间与考点场次，
/// 而这三样恰恰决定「报了名也进不去考场」。
pub const CONFIRM_ITEMS: &[(&str, &str)] = &[
  (
    "learning.exam-schedule-confirm-class",
    "learning.exam-schedule-confirm-class-desc",
  ),
  (
    "learning.exam-schedule-confirm-deadline",
    "learning.exam-schedule-confirm-deadline-desc",
  ),
  (
    "learning.exam-schedule-confirm-format",
    "learning.exam-schedule-confirm-format-desc",
  ),
  (
    "learning.exam-schedule-confirm-venue",
    "learning.exam-schedule-confirm-venue-desc",
  ),
  (
    "learning.exam-schedule-confirm-result",
    "learning.exam-schedule-confirm-result-desc",
  ),
];

/// 站内下一步：查到日期之后能接上的三个页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextStep {
  /// 标题（词典 key）。
  pub title: &'static str,
  /// 说明（词典 key）。
  pub desc: &'static str,
  /// 按钮文案（词典 key）。
  pub action: &'static str,
  /// 站内路径。
  pub href: &'static str,
}

/// 站内下一步，按「先排任务、再设提醒、最后摸底」的顺序。
pub const NEXT_STEPS: &[NextStep] = &[
  NextStep {
    title: "learning.exam-schedule-next-plan",
    desc: "learning.exam-schedule-next-plan-desc",
    action: "learning.exam-schedule-go-plan",
    href: "/study-calendar",
  },
  NextStep {
    title: "learning.exam-schedule-next-countdown",
    desc: "learning.exam-schedule-next-countdown-desc",
    action: "learning.exam-schedule-go-countdown",
    href: "/countdown",
  },
  NextStep {
    title: "learning.exam-schedule-next-mock",
    desc: "learning.exam-schedule-next-mock-desc",
    action: "learning.exam-schedule-go-mock",
    href: "/exam",
  },
];

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::*;

  /// 本模块的 key 前缀：词条住在 `learning` 域，漏写前缀会落到别的域文件里。
  const PREFIX: &str = "learning.exam-schedule";

  fn assert_key(key: &str) {
    assert!(key.starts_with(PREFIX), "词条 key 前缀不对：{key}");
    assert!(
      key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'),
      "词条 key 只允许小写字母、数字、连字符与点：{key}"
    );
    assert!(!key.ends_with('.'), "词条 key 不能以点结尾：{key}");
  }

  #[test]
  fn every_channel_has_a_translatable_name_and_entry() {
    assert!(!CHANNELS.is_empty());
    for c in CHANNELS {
      assert_key(c.name);
      assert_key(c.what);
      assert_key(c.entry);
    }
  }

  #[test]
  fn channel_keys_are_unique() {
    let mut seen = HashSet::new();
    for c in CHANNELS {
      assert!(seen.insert(c.name), "渠道名重复：{}", c.name);
      assert!(seen.insert(c.what), "渠道说明重复：{}", c.what);
    }
  }

  #[test]
  fn external_urls_are_https() {
    for c in CHANNELS {
      if let Some(url) = c.url {
        assert!(url.starts_with("https://"), "外链必须是 https：{url}");
      }
    }
  }

  #[test]
  fn confirm_items_are_key_pairs() {
    assert!(CONFIRM_ITEMS.len() >= 3);
    for (name, desc) in CONFIRM_ITEMS {
      assert_key(name);
      assert_key(desc);
      assert_ne!(name, desc);
    }
  }

  #[test]
  fn next_steps_point_at_internal_pages() {
    let mut seen = HashSet::new();
    for s in NEXT_STEPS {
      assert_key(s.title);
      assert_key(s.desc);
      assert_key(s.action);
      assert!(s.href.starts_with('/'), "站内路径要以 / 开头：{}", s.href);
      assert!(seen.insert(s.href), "站内路径重复：{}", s.href);
    }
  }
}
