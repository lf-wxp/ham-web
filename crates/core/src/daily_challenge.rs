//! 每日挑战：按日期确定性抽题、限时作答、记录每日成绩。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::exam::shuffle_in_place;

/// 每日挑战题数。
pub const CHALLENGE_COUNT: usize = 10;
/// 限时（分钟）。
pub const CHALLENGE_MINUTES: u32 = 10;
/// 成绩最多保留天数。
const KEEP_DAYS: usize = 90;

/// 某天的挑战结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyResult {
  pub correct: usize,
  pub total: usize,
}

impl DailyResult {
  /// 正确率（四舍五入百分比）。
  #[must_use]
  pub fn percent(self) -> i64 {
    if self.total == 0 {
      return 0;
    }
    (self.correct as f64 / self.total as f64 * 100.0).round() as i64
  }
}

/// 每日挑战成绩（`localStorage` 中 `daily-challenge`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyResults {
  #[serde(default)]
  pub days: BTreeMap<String, DailyResult>,
}

impl DailyResults {
  /// 记录某天成绩，并清理过旧记录。
  pub fn record(&mut self, date: &str, result: DailyResult) {
    self.days.insert(date.to_owned(), result);
    while self.days.len() > KEEP_DAYS {
      self.days.pop_first();
    }
  }
}

/// 判断闰年。
fn leap(year: i32) -> bool {
  (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 把 `YYYY-MM-DD` 往前推一天。
#[must_use]
pub fn prev_day(date: &str) -> Option<String> {
  let mut parts = date.split('-');
  let y: i32 = parts.next()?.parse().ok()?;
  let m: i32 = parts.next()?.parse().ok()?;
  let d: i32 = parts.next()?.parse().ok()?;
  if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
    return None;
  }
  let days_in_month = [
    31,
    if leap(y) { 29 } else { 28 },
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ];
  let (y, m, d) = if d > 1 {
    (y, m, d - 1)
  } else if m > 1 {
    (y, m - 1, days_in_month[(m - 2) as usize])
  } else {
    (y - 1, 12, 31)
  };
  Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// 当前连续完成每日挑战的天数：从 `today`（若今天未完成则从昨天）往前数连续日期。
#[must_use]
pub fn current_streak(days: &BTreeMap<String, DailyResult>, today: &str) -> usize {
  let start = if days.contains_key(today) {
    today.to_owned()
  } else {
    let Some(prev) = prev_day(today) else {
      return 0;
    };
    if !days.contains_key(&prev) {
      return 0;
    }
    prev
  };
  let mut n = 0;
  let mut cursor = start;
  loop {
    if !days.contains_key(&cursor) {
      break;
    }
    n += 1;
    let Some(prev) = prev_day(&cursor) else {
      break;
    };
    cursor = prev;
  }
  n
}

/// 历史最长连续挑战天数。
#[must_use]
pub fn longest_streak(days: &BTreeMap<String, DailyResult>) -> usize {
  let mut longest = 0;
  let mut run = 0;
  let mut prev: Option<String> = None;
  for date in days.keys() {
    let consecutive = prev
      .as_deref()
      .is_some_and(|p| prev_day(date).as_deref() == Some(p));
    run = if consecutive { run + 1 } else { 1 };
    longest = longest.max(run);
    prev = Some(date.clone());
  }
  longest
}

/// 日期字符串 → 确定性种子（FNV-1a 64）。
#[must_use]
pub fn seed_of(date: &str) -> u64 {
  let mut h: u64 = 0xcbf2_9ce4_8422_2325;
  for b in date.bytes() {
    h ^= u64::from(b);
    h = h.wrapping_mul(0x0100_0000_01b3);
  }
  h
}

/// 确定性 LCG：`seed` → `[0,1)` 随机序列（同一天抽到同一组题）。
pub fn lcg(seed: u64) -> impl FnMut() -> f64 {
  let mut s = seed | 1;
  move || {
    s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    #[allow(clippy::cast_precision_loss)]
    let v = (s >> 11) as f64 / (1u64 << 53) as f64;
    v
  }
}

/// 按日期确定性抽 `n` 题，返回下标（同一天抽到同一组题）。
#[must_use]
pub fn pick_daily(len: usize, date: &str, n: usize) -> Vec<usize> {
  let mut idx: Vec<usize> = (0..len).collect();
  let mut rng = lcg(seed_of(date));
  shuffle_in_place(&mut idx, &mut rng);
  idx.truncate(n.min(len));
  idx
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn seed_and_pick_are_deterministic() {
    assert_ne!(seed_of("2026-10-01"), seed_of("2026-10-02"));
    // 同一天抽到同一组题，不同天不同。
    let a = pick_daily(100, "2026-10-01", 10);
    let b = pick_daily(100, "2026-10-01", 10);
    let c = pick_daily(100, "2026-10-02", 10);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 10);
    assert_eq!(pick_daily(5, "2026-10-01", 10).len(), 5);
  }

  #[test]
  fn results_record_and_prune() {
    let mut r = DailyResults::default();
    r.record(
      "2026-10-01",
      DailyResult {
        correct: 8,
        total: 10,
      },
    );
    assert_eq!(r.days["2026-10-01"].percent(), 80);
    for d in 0..200 {
      r.record(
        &format!("2026-01-{d:02}"),
        DailyResult {
          correct: 1,
          total: 1,
        },
      );
    }
    assert!(r.days.len() <= KEEP_DAYS);
  }

  #[test]
  fn prev_day_handles_month_and_year_boundaries() {
    assert_eq!(prev_day("2026-10-01").as_deref(), Some("2026-09-30"));
    assert_eq!(prev_day("2026-01-01").as_deref(), Some("2025-12-31"));
    assert_eq!(prev_day("2024-03-01").as_deref(), Some("2024-02-29"));
    assert_eq!(prev_day("2023-03-01").as_deref(), Some("2023-02-28"));
    assert_eq!(prev_day("bad"), None);
  }

  #[test]
  fn streak_counts_consecutive_days() {
    let mut r = DailyResults::default();
    for d in ["2026-10-01", "2026-10-02", "2026-10-03"] {
      r.record(
        d,
        DailyResult {
          correct: 8,
          total: 10,
        },
      );
    }
    assert_eq!(current_streak(&r.days, "2026-10-03"), 3);
    assert_eq!(current_streak(&r.days, "2026-10-04"), 3);
    assert_eq!(current_streak(&r.days, "2026-10-05"), 0);
    assert_eq!(longest_streak(&r.days), 3);
  }
}
