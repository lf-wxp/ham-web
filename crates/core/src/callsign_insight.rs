//! 呼号查询的「我打过吗 / 什么方向 / 多远」。
//!
//! 三件事都来自站内数据：通联日志、本台网格、DXCC 实体中心。没有本台网格时只答
//! 「打过几次」，方位与距离如实留空——不拿「大概东边」这种话糊弄。

use crate::dxcc::lookup;
use crate::grid::{distance_bearing, lat_lon_from_grid};
use crate::logbook::LogEntry;

/// 最近一次通联。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastWorked {
  pub date: String,
  pub time: String,
  pub band: String,
}

/// 查询结果。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Insight {
  /// 日志里与该呼号匹配的通联数（忽略大小写与首尾空格）。
  pub worked: usize,
  /// 其中日期 + 时段最晚的一条。
  pub last: Option<LastWorked>,
  /// 从本台指向实体中心的方位角；没有本台网格或实体时为空。
  pub azimuth_deg: Option<f64>,
  /// 大圆距离（km）。
  pub distance_km: Option<f64>,
}

/// `a` 是否比 `b` 更晚。
///
/// 不能比字符串：`2026-9-5` 与 `2026-10-01` 逐字符比时第 6 位是 `'9'(57) > '1'(49)`，
/// 9 月 5 日会被判成**晚于** 10 月 1 日（`9:30` vs `10:00` 同理）；而日志里混用写法是
/// 常态（手改、备份恢复、第三方 ADIF，见 `logbook::normalized_date`）。一律换算成时刻。
/// 解析不出时刻的记录排在「已知」之后，免得一条畸形日期冒充「最近一次」。
fn newer(a: &LogEntry, b: &LogEntry) -> bool {
  match (a.minutes_on(), b.minutes_on()) {
    (Some(x), Some(y)) => x > y,
    (Some(_), None) => true,
    (None, Some(_)) => false,
    // 两条都解析不出：退回字符串比较，至少保证顺序是确定的。
    (None, None) => (&a.date, &a.time) > (&b.date, &b.time),
  }
}

/// 汇总一条呼号在本地日志与方位上的信息。
#[must_use]
pub fn insight(call: &str, home_grid: &str, entries: &[LogEntry]) -> Insight {
  let target = call.trim();
  // 空呼号不能去匹配日志里「呼号为空」的脏记录（ADIF 导入偶尔会带进来）。
  if target.is_empty() {
    return Insight::default();
  }

  let mut worked = 0usize;
  let mut last: Option<&LogEntry> = None;
  for entry in entries {
    if !entry.callsign.trim().eq_ignore_ascii_case(target) {
      continue;
    }
    worked += 1;
    if last.is_none_or(|prev| newer(entry, prev)) {
      last = Some(entry);
    }
  }

  let (azimuth_deg, distance_km) = match (lat_lon_from_grid(home_grid), lookup(target)) {
    (Some((home_lat, home_lon)), Some(entity)) => {
      let (km, azimuth) = distance_bearing(home_lat, home_lon, entity.lat, entity.lon);
      (Some(azimuth), Some(km))
    }
    _ => (None, None),
  };

  Insight {
    worked,
    last: last.map(|entry| LastWorked {
      date: entry.date.clone(),
      time: entry.time.clone(),
      band: entry.band_label(),
    }),
    azimuth_deg,
    distance_km,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(call: &str, date: &str, time: &str, freq: &str) -> LogEntry {
    LogEntry {
      callsign: call.to_owned(),
      date: date.to_owned(),
      time: time.to_owned(),
      freq: freq.to_owned(),
      ..LogEntry::default()
    }
  }

  #[test]
  fn empty_log_reports_nothing_worked() {
    let got = insight("JA1ABC", "OM89", &[]);
    assert_eq!(got.worked, 0);
    assert!(got.last.is_none());
    // 网格与实体都有效时，方位与距离照样给。
    assert!(got.azimuth_deg.is_some());
    assert!(got.distance_km.is_some());
  }

  #[test]
  fn counts_matches_and_picks_the_latest() {
    let entries = vec![
      entry("JA1ABC", "2025-03-01", "10:00", "14.025"),
      entry("JA1ABC", "2025-06-01", "22:30", "7.025"),
      entry("K1ABC", "2025-07-01", "01:00", "21.025"),
    ];
    let got = insight("ja1abc ", "OM89", &entries);
    assert_eq!(got.worked, 2, "大小写与首尾空格不该影响匹配");
    let last = got.last.expect("有通联");
    assert_eq!(last.date, "2025-06-01");
    assert_eq!(last.time, "22:30");
    assert_eq!(last.band, "40m");
  }

  #[test]
  fn same_day_compares_by_time() {
    let entries = vec![
      entry("JA1ABC", "2025-06-01", "23:10", "14.025"),
      entry("JA1ABC", "2025-06-01", "02:10", "14.025"),
    ];
    let got = insight("JA1ABC", "OM89", &entries);
    assert_eq!(got.last.expect("有通联").time, "23:10");
  }

  #[test]
  fn mixed_date_spellings_still_pick_the_latest() {
    // `2026-9-5` 与 `2026-10-01`：逐字符比时第 6 位 `'9' > '1'`，9 月 5 日会被判成更晚，
    // 「最近一次」于是给出错误答案。
    let entries = vec![
      entry("JA1ABC", "2026-9-5", "9:30", "14.025"),
      entry("JA1ABC", "2026-10-01", "10:00", "7.025"),
    ];
    let got = insight("JA1ABC", "OM89", &entries);
    assert_eq!(got.last.expect("有通联").date, "2026-10-01");
  }

  #[test]
  fn unparsable_date_does_not_impersonate_the_latest() {
    let entries = vec![
      entry("JA1ABC", "2025-06-01", "10:00", "14.025"),
      entry("JA1ABC", "", "23:59", "7.025"),
    ];
    assert_eq!(
      insight("JA1ABC", "OM89", &entries)
        .last
        .expect("有通联")
        .date,
      "2025-06-01"
    );
  }

  #[test]
  fn without_home_grid_bearing_is_unknown() {
    let got = insight(
      "JA1ABC",
      "",
      &[entry("JA1ABC", "2025-06-01", "10:00", "14.025")],
    );
    assert_eq!(got.worked, 1, "没有网格不影响「打过几次」");
    assert!(got.azimuth_deg.is_none());
    assert!(got.distance_km.is_none());
  }

  #[test]
  fn bearing_and_distance_point_at_the_entity_centre() {
    // OM89（北京附近）→ 日本：东偏南，约 2000 km。
    let got = insight("JA1ABC", "OM89", &[]);
    let azimuth = got.azimuth_deg.expect("有效网格与实体");
    let distance = got.distance_km.expect("有效网格与实体");
    assert!((60.0..140.0).contains(&azimuth), "{azimuth}");
    assert!((1500.0..2700.0).contains(&distance), "{distance}");
  }
}
