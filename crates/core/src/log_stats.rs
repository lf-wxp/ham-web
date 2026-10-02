//! 通联日志的可视化统计：UTC 时段 × 波段的通联密度热力图。

use std::collections::BTreeMap;

use crate::logbook::LogEntry;

/// 日志波段的展示顺序（低频到高频）。
pub const BAND_ORDER: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

/// 统计每个波段在 24 个 UTC 小时内的通联数量。
///
/// 返回按 [`BAND_ORDER`] 排序的 `(波段, [24 小时计数])` 列表，未在排序表内出现的波段
/// （如「其他」）按字母序排在其后。忽略空时间与空波段的记录。
#[must_use]
pub fn hour_band_heatmap(entries: &[LogEntry]) -> Vec<(String, [u32; 24])> {
  let mut map: BTreeMap<String, [u32; 24]> = BTreeMap::new();
  for e in entries {
    let band = e.band_label();
    if band.is_empty() {
      continue;
    }
    let Some(hour) = e
      .time
      .get(..2)
      .and_then(|s| s.parse::<usize>().ok())
      .filter(|&h| h < 24)
    else {
      continue;
    };
    map.entry(band).or_insert([0; 24])[hour] += 1;
  }

  let mut out: Vec<(String, [u32; 24])> = BAND_ORDER
    .iter()
    .filter_map(|b| map.remove(*b).map(|v| ((*b).to_owned(), v)))
    .collect();
  out.extend(map);
  out
}

/// 按日期统计 QSO 数量，key 为 `YYYY-MM-DD`（忽略日期字段为空或格式不完整的记录）。
///
/// 返回按日期升序的映射，供「日历热力图」按天着色。
#[must_use]
pub fn daily_qso_counts(entries: &[LogEntry]) -> BTreeMap<String, u32> {
  let mut map: BTreeMap<String, u32> = BTreeMap::new();
  for e in entries {
    let Some(date) = e.date.get(..10).filter(|d| d.len() == 10) else {
      continue;
    };
    *map.entry(date.to_owned()).or_default() += 1;
  }
  map
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::logbook::LogEntry;

  fn entry(band: &str, time: &str) -> LogEntry {
    LogEntry {
      band: band.to_owned(),
      time: time.to_owned(),
      ..LogEntry::default()
    }
  }

  #[test]
  fn groups_by_band_and_hour() {
    let entries = vec![
      entry("20m", "08:30"),
      entry("20m", "08:45"),
      entry("20m", "09:00"),
      entry("40m", "08:10"),
    ];
    let heat = hour_band_heatmap(&entries);
    // 按 BAND_ORDER：40m 在 20m 之前
    assert_eq!(heat[0].0, "40m");
    assert_eq!(heat[0].1[8], 1);
    assert_eq!(heat[1].0, "20m");
    assert_eq!(heat[1].1[8], 2);
    assert_eq!(heat[1].1[9], 1);
  }

  #[test]
  fn skips_invalid_time() {
    let entries = vec![
      entry("20m", ""),
      entry("20m", "99:00"),
      entry("20m", "07:00"),
    ];
    let heat = hour_band_heatmap(&entries);
    assert_eq!(heat.len(), 1);
    assert_eq!(heat[0].1[7], 1);
  }

  #[test]
  fn counts_daily_qso() {
    let mut a = entry("20m", "08:30");
    a.date = "2024-05-01".to_owned();
    let mut b = entry("20m", "09:00");
    b.date = "2024-05-01".to_owned();
    let mut c = entry("40m", "08:10");
    c.date = "2024-05-02".to_owned();
    let mut invalid = entry("40m", "08:10");
    invalid.date = "bad".to_owned();
    let counts = daily_qso_counts(&[a, b, c, invalid]);
    assert_eq!(counts.get("2024-05-01"), Some(&2));
    assert_eq!(counts.get("2024-05-02"), Some(&1));
    assert!(!counts.contains_key("2024-05-03"));
  }
}
