//! 业余无线电活动日历：展会 / 火腿节与年度通联活动（每年固定时段的重复事件）。
//!
//! 竞赛日历见 [`crate::contest_calendar`]；本模块补充「展会 · 火腿节」与「通联活动」两类，
//! 与竞赛一起构成完整的活动日历。日期按常见安排推算，具体以主办方公告为准。

/// 一项年度活动。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HamEvent {
  /// 唯一标识。
  pub id: &'static str,
  /// 活动名称。
  pub name: &'static str,
  /// 类别（展会 · 火腿节 / 通联活动）。
  pub kind: &'static str,
  /// 起始月份（1～12）。
  pub month: u32,
  /// 起始日（1～31）。
  pub day: u32,
  /// 持续天数。
  pub duration_days: u32,
  /// 地点（国家 · 城市，或「全球」）。
  pub location: &'static str,
  /// 官网等外部链接；无则为空串。
  pub source: &'static str,
  /// 简介。
  pub desc: &'static str,
}

/// 活动类别。
pub const EVENT_KINDS: &[&str] = &["展会 · 火腿节", "通联活动"];

/// 年度活动（按月份、日期排序；日期为常见安排，每年略有浮动）。
pub const HAM_EVENTS: &[HamEvent] = &[
  HamEvent {
    id: "winter-field-day",
    name: "冬季野外设台日",
    kind: "通联活动",
    month: 1,
    day: 25,
    duration_days: 1,
    location: "全球",
    source: "",
    desc: "冬季户外架台与应急通信演练（Winter Field Day）。",
  },
  HamEvent {
    id: "parks-on-the-air",
    name: "POTA 支持公园日",
    kind: "通联活动",
    month: 4,
    day: 18,
    duration_days: 2,
    location: "全球",
    source: "",
    desc: "在公园设台通联的全球性活动，详见「SOTA / POTA」。",
  },
  HamEvent {
    id: "hamvention",
    name: "Dayton Hamvention",
    kind: "展会 · 火腿节",
    month: 5,
    day: 21,
    duration_days: 3,
    location: "美国 · 俄亥俄州 Xenia",
    source: "https://hamvention.org/",
    desc: "全球规模最大的业余无线电展销会，每年 5 月第 3 个周末。",
  },
  HamEvent {
    id: "museum-ships",
    name: "博物馆舰船周末",
    kind: "通联活动",
    month: 6,
    day: 6,
    duration_days: 2,
    location: "全球",
    source: "",
    desc: "在退役军舰、潜艇上设台通联的年度活动。",
  },
  HamEvent {
    id: "ham-radio",
    name: "HAM RADIO Friedrichshafen",
    kind: "展会 · 火腿节",
    month: 6,
    day: 25,
    duration_days: 3,
    location: "德国 · 腓特烈港",
    source: "https://www.hamradio-friedrichshafen.de/",
    desc: "欧洲最大的业余无线电展会。",
  },
  HamEvent {
    id: "field-day",
    name: "ARRL Field Day",
    kind: "通联活动",
    month: 6,
    day: 26,
    duration_days: 1,
    location: "全球",
    source: "",
    desc: "北美最大的野外应急设台演练，连续 24 小时通联。",
  },
  HamEvent {
    id: "iota-contest",
    name: "IOTA 海岛通联活动",
    kind: "通联活动",
    month: 7,
    day: 25,
    duration_days: 1,
    location: "全球海岛",
    source: "",
    desc: "海岛通联（Islands on the Air）年度活动，详见「IOTA」。",
  },
  HamEvent {
    id: "lighthouse-weekend",
    name: "灯塔与灯船周末",
    kind: "通联活动",
    month: 8,
    day: 15,
    duration_days: 2,
    location: "全球",
    source: "",
    desc: "在世界各地灯塔、灯船设台通联的年度活动。",
  },
  HamEvent {
    id: "tokyo-ham-fair",
    name: "东京 Ham Fair",
    kind: "展会 · 火腿节",
    month: 8,
    day: 22,
    duration_days: 2,
    location: "日本 · 东京",
    source: "https://www.jarl.org/",
    desc: "日本 JARL 主办，亚洲最具规模的业余无线电展。",
  },
  HamEvent {
    id: "jota",
    name: "JOTA 空中大露营",
    kind: "通联活动",
    month: 10,
    day: 17,
    duration_days: 1,
    location: "全球",
    source: "",
    desc: "童子军与业余无线电爱好者通过电台交流的年度活动。",
  },
];

/// 给定当前 UTC 年月日，返回某项活动下一次开始的 UTC 年月日（`(year, month, day)`）。
#[must_use]
pub fn next_start(event: &HamEvent, now: (i32, u32, u32)) -> (i32, u32, u32) {
  let now_year = now.0;
  let this_year = (now_year, event.month, event.day);
  if this_year >= now {
    this_year
  } else {
    (now_year + 1, event.month, event.day)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn events_sorted_and_populated() {
    assert!(HAM_EVENTS.len() >= 8);
    for w in HAM_EVENTS.windows(2) {
      let (a, b) = (w[0], w[1]);
      assert!(
        (a.month, a.day) <= (b.month, b.day),
        "活动应按月份、日期排序"
      );
    }
    for e in HAM_EVENTS {
      assert!(!e.id.is_empty());
      assert!(!e.name.is_empty());
      assert!(!e.location.is_empty());
      assert!(!e.desc.is_empty());
      assert!(EVENT_KINDS.contains(&e.kind), "未知类别：{}", e.kind);
      assert!((1..=12).contains(&e.month));
      assert!((1..=31).contains(&e.day));
      assert!(e.source.is_empty() || e.source.starts_with("https://"));
    }
  }

  #[test]
  fn next_start_rolls_over_year() {
    let hamvention = HAM_EVENTS.iter().find(|e| e.id == "hamvention").unwrap();
    assert_eq!(next_start(hamvention, (2026, 12, 1)), (2027, 5, 21));
    assert_eq!(next_start(hamvention, (2026, 1, 1)), (2026, 5, 21));
    assert_eq!(next_start(hamvention, (2026, 5, 21)), (2026, 5, 21));
  }
}
