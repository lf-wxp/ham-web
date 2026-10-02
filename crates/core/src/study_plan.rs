//! 备考计划：按考试日期与剩余题量安排每日目标，并记录每天的作答量。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bank::Bank;

/// 每日记录最多保留的天数。
pub const KEEP_DAYS: usize = 60;
/// 距考试不超过这么多天时，每天建议做一套模拟考试。
pub const MOCK_EXAM_DAYS: i64 = 7;

/// 某一天的作答量。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayTally {
  /// 作答题数（含重复）。
  #[serde(default)]
  pub answered: u32,
  /// 其中第一次做的题数。
  #[serde(default)]
  pub new: u32,
  /// 当日累计作答耗时（毫秒）。
  #[serde(default)]
  pub duration_ms: u64,
}

impl DayTally {
  /// 作答耗时（分钟，保留一位小数语义由调用方格式化）。
  #[must_use]
  pub fn minutes(self) -> f64 {
    self.duration_ms as f64 / 60_000.0
  }
}

/// 按本地日期（`YYYY-MM-DD`）记录的每日作答量（`localStorage` 中 `study-daily`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyLog {
  #[serde(default)]
  pub days: BTreeMap<String, DayTally>,
}

impl DailyLog {
  /// 记一次作答；`new` 表示这道题以前没做过，`duration_ms` 为本次作答耗时。
  pub fn add(&mut self, day: &str, new: bool, duration_ms: u64) {
    let t = self.days.entry(day.to_owned()).or_default();
    t.answered += 1;
    t.new += u32::from(new);
    t.duration_ms += duration_ms;
    while self.days.len() > KEEP_DAYS {
      self.days.pop_first();
    }
  }

  /// 某天的作答量。
  #[must_use]
  pub fn on(&self, day: &str) -> DayTally {
    self.days.get(day).copied().unwrap_or_default()
  }

  /// 截至 `today` 的最近 `n` 天（含没有作答的日子），按日期升序。
  #[must_use]
  pub fn recent(&self, today: &str, n: usize) -> Vec<(String, DayTally)> {
    let Some(end) = day_number(today) else {
      return Vec::new();
    };
    (0..n as i64)
      .rev()
      .map(|back| {
        let day = format_day(end - back);
        let t = self.on(&day);
        (day, t)
      })
      .collect()
  }
}

/// 备考计划（`localStorage` 中 `learning-plan`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyPlan {
  /// 考试日期 `YYYY-MM-DD`。
  #[serde(default)]
  pub target_date: String,
  /// 报考类别（空则取作答最多的题库）。
  #[serde(default)]
  pub bank: Option<Bank>,
  /// 每日学习提醒时间 `HH:MM`（24 小时制，空则关闭提醒）。
  #[serde(default)]
  pub reminder_time: String,
}

impl StudyPlan {
  /// 距考试的天数（考试当天为 0，已过为负）。
  #[must_use]
  pub fn days_left(&self, today: &str) -> Option<i64> {
    Some(day_number(&self.target_date)? - day_number(today)?)
  }
}

/// 备考阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
  /// 刷新题为主。
  Learn,
  /// 新题已刷完或临近考试：以错题和模拟考试为主。
  Sprint,
  /// 考试当天。
  ExamDay,
  /// 考试日期已过。
  Over,
}

/// 今日目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyGoal {
  pub phase: Phase,
  pub days_left: i64,
  /// 今日新题目标（含今天已做的新题）。
  pub new_target: usize,
  /// 今日还要复习的错题数。
  pub review: usize,
  /// 今日建议做一套模拟考试。
  pub mock_exam: bool,
}

/// 冲刺留出的天数：最后几天不再安排新题，专心复习与模拟考试。
fn sprint_days(days_left: i64) -> i64 {
  match days_left {
    d if d >= 14 => 3,
    d if d >= 5 => 1,
    _ => 0,
  }
}

/// 计算今日目标。
///
/// `unseen` 为题库中还没做过的题数，`new_today` 为今天已做的新题数（计入目标，避免越做目标越少），
/// `due` 为今天待复习的错题数。
#[must_use]
pub fn daily_goal(days_left: i64, unseen: usize, new_today: usize, due: usize) -> DailyGoal {
  if days_left < 0 {
    return DailyGoal {
      phase: Phase::Over,
      days_left,
      new_target: 0,
      review: 0,
      mock_exam: false,
    };
  }
  if days_left == 0 {
    return DailyGoal {
      phase: Phase::ExamDay,
      days_left,
      new_target: 0,
      review: due,
      mock_exam: false,
    };
  }
  let remaining = unseen + new_today;
  let learn_days = (days_left - sprint_days(days_left)).max(1) as usize;
  let in_sprint = unseen == 0 || days_left <= sprint_days(days_left);
  let new_target = if in_sprint && unseen > 0 {
    remaining
  } else {
    remaining.div_ceil(learn_days)
  };
  DailyGoal {
    phase: if in_sprint {
      Phase::Sprint
    } else {
      Phase::Learn
    },
    days_left,
    new_target,
    review: due,
    mock_exam: days_left <= MOCK_EXAM_DAYS || unseen == 0,
  }
}

/// `YYYY-MM-DD` → 自 1970-01-01 起的天数。
#[must_use]
pub fn day_number(s: &str) -> Option<i64> {
  let mut it = s.trim().splitn(3, '-');
  let y: i64 = it.next()?.parse().ok()?;
  let m: i64 = it.next()?.parse().ok()?;
  let d: i64 = it.next()?.parse().ok()?;
  if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
    return None;
  }
  // Howard Hinnant 的 days_from_civil
  let y = if m <= 2 { y - 1 } else { y };
  let era = y.div_euclid(400);
  let yoe = y - era * 400;
  let mp = (m + 9) % 12;
  let doy = (153 * mp + 2) / 5 + d - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  Some(era * 146_097 + doe - 719_468)
}

/// 天数 → `YYYY-MM-DD`。
#[must_use]
pub fn format_day(days: i64) -> String {
  let z = days + 719_468;
  let era = z.div_euclid(146_097);
  let doe = z - era * 146_097;
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = doy - (153 * mp + 2) / 5 + 1;
  let m = if mp < 10 { mp + 3 } else { mp - 9 };
  let y = yoe + era * 400 + i64::from(m <= 2);
  format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn day_numbers_round_trip() {
    assert_eq!(day_number("1970-01-01"), Some(0));
    assert_eq!(
      day_number("2024-03-01").unwrap() - day_number("2024-02-28").unwrap(),
      2
    );
    for s in ["2000-02-29", "2026-10-01", "1999-12-31"] {
      assert_eq!(format_day(day_number(s).unwrap()), s);
    }
    assert_eq!(day_number("2026-13-01"), None);
    assert_eq!(day_number(""), None);
  }

  #[test]
  fn spreads_unseen_over_learning_days() {
    // 30 天：留 3 天冲刺，600 题 / 27 天 = 23 题
    let g = daily_goal(30, 600, 0, 5);
    assert_eq!(g.phase, Phase::Learn);
    assert_eq!(g.new_target, 23);
    assert_eq!(g.review, 5);
    assert!(!g.mock_exam);
    // 今天已做的新题计入目标，目标不随做题变小
    assert_eq!(daily_goal(30, 590, 10, 0).new_target, 23);
  }

  #[test]
  fn sprint_and_exam_day() {
    let g = daily_goal(3, 0, 0, 2);
    assert_eq!(g.phase, Phase::Sprint);
    assert_eq!(g.new_target, 0);
    assert!(g.mock_exam);
    // 冲刺期还有没做的题：今天全部做完
    assert_eq!(daily_goal(1, 40, 0, 0).new_target, 40);
    assert_eq!(daily_goal(0, 40, 0, 3).phase, Phase::ExamDay);
    assert_eq!(daily_goal(-1, 40, 0, 3).phase, Phase::Over);
  }

  #[test]
  fn daily_log_counts_and_prunes() {
    let mut log = DailyLog::default();
    log.add("2026-10-01", true, 5000);
    log.add("2026-10-01", false, 3000);
    assert_eq!(
      log.on("2026-10-01"),
      DayTally {
        answered: 2,
        new: 1,
        duration_ms: 8000
      }
    );
    let recent = log.recent("2026-10-02", 3);
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].0, "2026-09-30");
    assert_eq!(recent[1].1.answered, 2);
    for d in 0..100 {
      log.add(&format_day(d), false, 0);
    }
    assert_eq!(log.days.len(), KEEP_DAYS);
  }

  #[test]
  fn plan_days_left_and_legacy_json() {
    let plan: StudyPlan =
      serde_json::from_str(r#"{"target_date":"2026-10-11","target_count":500}"#).unwrap();
    assert_eq!(plan.bank, None);
    assert_eq!(plan.days_left("2026-10-01"), Some(10));
  }
}
