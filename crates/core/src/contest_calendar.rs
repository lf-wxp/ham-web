//! 全球主要业余无线电竞赛日历（数据 + 下一次发生时间计算）。

/// 一项竞赛。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestEvent {
  /// 唯一标识。
  pub id: &'static str,
  /// 竞赛名称。
  pub name: &'static str,
  /// 模式（SSB / CW / RTTY / 混合 / 其他）。
  pub mode: &'static str,
  /// 起始月份（1–12）。
  pub month: u32,
  /// 起始日（1–31，UTC）。
  pub day: u32,
  /// 持续时间（天，通常是 48 小时即 2 天）。
  pub duration_days: u32,
  /// 交换信息。
  pub exchange: &'static str,
  /// 简介。
  pub desc: &'static str,
  /// 对应的竞赛录入模板 ID（`/contest-log?contest=`），无对应模板时为 `None`。
  pub contest_id: Option<&'static str>,
}

/// 主要国际竞赛（按月份顺序；日期为 UTC 起始，以主办方公告为准）。
pub const CONTEST_CALENDAR: &[ContestEvent] = &[
  ContestEvent {
    id: "cq-wpx-rtty",
    name: "CQ WPX RTTY",
    mode: "RTTY",
    month: 2,
    day: 14,
    duration_days: 2,
    exchange: "序号",
    desc: "RTTY 模式全球大赛，按呼号前缀数量计分。",
    contest_id: None,
  },
  ContestEvent {
    id: "arrl-dx-cw",
    name: "ARRL DX CW",
    mode: "CW",
    month: 2,
    day: 21,
    duration_days: 2,
    exchange: "功率 + 州/省",
    desc: "美国以外台仅与 DXCC 实体通联计分。",
    contest_id: Some("ARRL-DX-CW"),
  },
  ContestEvent {
    id: "arrl-dx-ssb",
    name: "ARRL DX SSB",
    mode: "SSB",
    month: 3,
    day: 7,
    duration_days: 2,
    exchange: "功率 + 州/省",
    desc: "ARRL DX 的 SSB 场次。",
    contest_id: Some("ARRL-DX-SSB"),
  },
  ContestEvent {
    id: "russian-dx",
    name: "俄罗斯 DX",
    mode: "混合",
    month: 3,
    day: 15,
    duration_days: 1,
    exchange: "序号 + CQ 分区",
    desc: "俄罗斯 DX 赛，交换序号，对方报 CQ 分区。",
    contest_id: Some("RUSSIAN-DX"),
  },
  ContestEvent {
    id: "cq-wpx-ssb",
    name: "CQ WPX SSB",
    mode: "SSB",
    month: 3,
    day: 28,
    duration_days: 2,
    exchange: "序号",
    desc: "按不同呼号前缀数量计分，交换序号。",
    contest_id: Some("CQ-WPX-SSB"),
  },
  ContestEvent {
    id: "jidx-cw",
    name: "JIDX CW",
    mode: "CW",
    month: 4,
    day: 12,
    duration_days: 2,
    exchange: "CQ 分区 + 序号",
    desc: "日本国际 DX，与 JA 台通联，乘数按 JA 都道府县。",
    contest_id: Some("JIDX-CW"),
  },
  ContestEvent {
    id: "cq-wpx-cw",
    name: "CQ WPX CW",
    mode: "CW",
    month: 5,
    day: 30,
    duration_days: 2,
    exchange: "序号",
    desc: "CW 模式，按呼号前缀数量计分。",
    contest_id: Some("CQ-WPX-CW"),
  },
  ContestEvent {
    id: "all-asian-cw",
    name: "All Asian DX CW",
    mode: "CW",
    month: 6,
    day: 21,
    duration_days: 2,
    exchange: "年龄 + 序号",
    desc: "亚洲 DX 赛，亚洲台与洲外通联计分。",
    contest_id: Some("ALL-ASIAN-CW"),
  },
  ContestEvent {
    id: "arrl-field-day",
    name: "ARRL Field Day",
    mode: "混合",
    month: 6,
    day: 27,
    duration_days: 1,
    exchange: "类别 + ARRL/RAC 分区",
    desc: "野外应急设台 24 小时，北美最大型活动。",
    contest_id: None,
  },
  ContestEvent {
    id: "iaru-hf",
    name: "IARU HF 锦标赛",
    mode: "混合",
    month: 7,
    day: 11,
    duration_days: 1,
    exchange: "ITU 分区 + 协会缩写",
    desc: "交换 ITU 分区号与参赛协会缩写。",
    contest_id: Some("IARU-HF"),
  },
  ContestEvent {
    id: "wae-cw",
    name: "WAE DX CW",
    mode: "CW",
    month: 8,
    day: 9,
    duration_days: 2,
    exchange: "序号 + QTC",
    desc: "欧洲 DX 赛，支持 QTC 转发。",
    contest_id: Some("WAE-CW"),
  },
  ContestEvent {
    id: "all-asian-ssb",
    name: "All Asian DX SSB",
    mode: "SSB",
    month: 9,
    day: 6,
    duration_days: 2,
    exchange: "年龄 + 序号",
    desc: "亚洲 DX 赛的 SSB 场次。",
    contest_id: Some("ALL-ASIAN-SSB"),
  },
  ContestEvent {
    id: "wae-ssb",
    name: "WAE DX SSB",
    mode: "SSB",
    month: 9,
    day: 13,
    duration_days: 2,
    exchange: "序号 + QTC",
    desc: "欧洲 DX 赛的 SSB 场次。",
    contest_id: Some("WAE-SSB"),
  },
  ContestEvent {
    id: "cq-ww-rtty",
    name: "CQ WW RTTY",
    mode: "RTTY",
    month: 9,
    day: 26,
    duration_days: 2,
    exchange: "CQ 分区",
    desc: "RTTY 全球大赛，交换 CQ 分区与序号。",
    contest_id: None,
  },
  ContestEvent {
    id: "cq-ww-ssb",
    name: "CQ WW SSB",
    mode: "SSB",
    month: 10,
    day: 24,
    duration_days: 2,
    exchange: "CQ 分区",
    desc: "全球最大 SSB 竞赛，交换 CQ 分区与序号。",
    contest_id: Some("CQ-WW-SSB"),
  },
  ContestEvent {
    id: "jidx-ssb",
    name: "JIDX SSB",
    mode: "SSB",
    month: 11,
    day: 8,
    duration_days: 2,
    exchange: "CQ 分区 + 序号",
    desc: "日本国际 DX 的 SSB 场次。",
    contest_id: Some("JIDX-SSB"),
  },
  ContestEvent {
    id: "cq-ww-cw",
    name: "CQ WW CW",
    mode: "CW",
    month: 11,
    day: 28,
    duration_days: 2,
    exchange: "CQ 分区",
    desc: "全球最大 CW 竞赛，交换 CQ 分区与序号。",
    contest_id: Some("CQ-WW-CW"),
  },
  ContestEvent {
    id: "arrl-10m",
    name: "ARRL 十米竞赛",
    mode: "混合",
    month: 12,
    day: 12,
    duration_days: 2,
    exchange: "州/省 + 序号",
    desc: "仅限 10 米波段，交换州 / 省 / 序号。",
    contest_id: None,
  },
];

/// 给定当前 UTC 年月日，返回某项竞赛下一次开始的 UTC 年月日（`(year, month, day)`）。
#[must_use]
pub fn next_start(event: &ContestEvent, now: (i32, u32, u32)) -> (i32, u32, u32) {
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
  fn calendar_is_sorted_and_populated() {
    assert!(CONTEST_CALENDAR.len() >= 10);
    for w in CONTEST_CALENDAR.windows(2) {
      let (a, b) = (w[0], w[1]);
      assert!(a.month <= b.month, "calendar should be month-sorted");
    }
    for e in CONTEST_CALENDAR {
      assert!(!e.name.is_empty());
      assert!(!e.exchange.is_empty());
      assert!(!e.desc.is_empty());
      assert!((1..=12).contains(&e.month));
      assert!((1..=31).contains(&e.day));
    }
  }

  #[test]
  fn contest_ids_reference_known_templates() {
    // 带模板的日历项必须能在竞赛录入模板里找到。
    for e in CONTEST_CALENDAR.iter().filter(|e| e.contest_id.is_some()) {
      assert!(
        crate::contest::contest(e.contest_id.unwrap()).is_some(),
        "unknown contest template: {}",
        e.contest_id.unwrap()
      );
    }
  }

  #[test]
  fn next_start_rolls_over_year() {
    let cq_ww_cw = CONTEST_CALENDAR
      .iter()
      .find(|e| e.id == "cq-ww-cw")
      .unwrap();
    // 11/28 之后 → 明年
    assert_eq!(next_start(cq_ww_cw, (2026, 12, 1)), (2027, 11, 28));
    // 之前 → 今年
    assert_eq!(next_start(cq_ww_cw, (2026, 1, 1)), (2026, 11, 28));
    // 当天 → 今年当天
    assert_eq!(next_start(cq_ww_cw, (2026, 11, 28)), (2026, 11, 28));
  }
}
