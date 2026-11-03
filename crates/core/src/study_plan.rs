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
    // 饱和加：这三个数来自 `localStorage` 反序列化，被手改成接近上限的值时，
    // 普通 `+=` 在 debug 构建会 panic、release 会回绕成很小的数。
    t.answered = t.answered.saturating_add(1);
    t.new = t.new.saturating_add(u32::from(new));
    t.duration_ms = t.duration_ms.saturating_add(duration_ms);
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

/// 「信号瀑布」的强度档位上限。
pub const INTENSITY_MAX: u8 = 5;

/// 把一组每日作答量折算成瀑布图的强度档位（`0` = 当天未开台，`1..=5` 由浅到深）。
///
/// 基准优先取 `goal`（当天目标题量）；没有目标时退回这一组里的最大值。
/// 首页是本地快照，不为了给几格上色去异步加载题库算「今日目标」—— 那会让首屏多一次请求，
/// 而这里的视觉目的是「哪天亮、哪天暗」，不是「今天完成了几成」。
/// 全组为 0（从未作答）时逐项返回 0，让视图画空状态，而不是画一排假的浅色格子。
#[must_use]
pub fn intensity_levels(days: impl IntoIterator<Item = u32>, goal: Option<u32>) -> Vec<u8> {
  let counts: Vec<u32> = days.into_iter().collect();
  let base = goal
    .filter(|g| *g > 0)
    .or_else(|| counts.iter().copied().max())
    .unwrap_or(0);
  if base == 0 {
    return vec![0; counts.len()];
  }
  counts
    .into_iter()
    .map(|n| {
      if n == 0 {
        return 0;
      }
      // 向上取整：只要有作答就至少亮一档，否则「做了一道」与「没做」在图上一样。
      let level = (u64::from(n) * u64::from(INTENSITY_MAX)).div_ceil(u64::from(base));
      u8::try_from(level.min(u64::from(INTENSITY_MAX))).unwrap_or(INTENSITY_MAX)
    })
    .collect()
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
  // 只收纯数字：`str::parse::<i64>` 会放过 `+2026` 这类带符号的写法，日期格式契约里没有它。
  let mut field = || -> Option<i64> {
    let part = it.next()?;
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
      return None;
    }
    part.parse().ok()
  };
  let y = field()?;
  let m = field()?;
  let d = field()?;
  // 年份也要限：`era * 146_097` 要 6×10^15 年才溢出 i64，但畸形输入（上游 JSON 里
  // 十七位的年份、手改过的目标日期）会让 debug 构建直接 panic —— 日期格式契约是 4 位年。
  if !(1..=9999).contains(&y) {
    return None;
  }
  if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
    // 按当月真实天数判：`2026-02-31` 以前会被接受并悄悄算成 3 月 3 日。
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

/// 公历某年某月的天数（`m` 为 1–12）。
fn days_in_month(y: i64, m: i64) -> i64 {
  match m {
    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
    4 | 6 | 9 | 11 => 30,
    _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
    _ => 28,
  }
}

/// 日期算法的天数取值范围（约 ±11 亿年）：超出就夹住，避免 `days + 719_468` 之类溢出。
const DAY_RANGE: i64 = 1 << 32;

/// 天数 → `YYYY-MM-DD`。`days` 夹在 ±[`DAY_RANGE`] 内（正常日期远在其中）。
#[must_use]
pub fn format_day(days: i64) -> String {
  let z = days.clamp(-DAY_RANGE, DAY_RANGE) + 719_468;
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
    // 年份越界也判 None：畸形输入不能把日期算法推进溢出（上游 JSON 里的 time 会直接喂进来）。
    assert_eq!(day_number("99999-01-01"), None);
    assert_eq!(day_number("0000-01-01"), None);
    assert_eq!(day_number("-2026-01-01"), None);
    // 月内天数按真实历法判：以前 `2026-02-31` 会被算成 3 月 3 日。
    assert_eq!(day_number("2026-02-31"), None);
    assert_eq!(day_number("2026-04-31"), None);
    assert_eq!(day_number("2025-02-29"), None);
    assert!(day_number("2024-02-29").is_some());
    assert_eq!(day_number("1900-02-29"), None, "1900 不是闰年");
    assert!(day_number("2000-02-29").is_some(), "2000 是闰年");
    // 带符号 / 空段不是合法写法。
    assert_eq!(day_number("+2026-+10-+01"), None);
    assert_eq!(day_number("2026--01"), None);
    assert_eq!(day_number("2026-10-"), None);
  }

  #[test]
  fn format_day_survives_extreme_input() {
    // 不 panic、也不回绕出一个看似合理的日期：夹住后是确定的字符串。
    assert_eq!(format_day(i64::MAX), format_day(DAY_RANGE));
    assert_eq!(format_day(i64::MIN), format_day(-DAY_RANGE));
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
  fn daily_log_keeps_the_newest_days() {
    // 只断言条数会放过「删错了一头」：必须验证留下的是**最新**的 KEEP_DAYS 天。
    let mut log = DailyLog::default();
    for d in 0..100 {
      log.add(&format_day(d), false, 0);
    }
    assert_eq!(log.days.len(), KEEP_DAYS);
    let newest = 99;
    let oldest_kept = newest - KEEP_DAYS as i64 + 1;
    assert_eq!(log.days.keys().next(), Some(&format_day(oldest_kept)));
    assert_eq!(log.days.keys().next_back(), Some(&format_day(newest)));
    assert!(!log.days.contains_key(&format_day(oldest_kept - 1)));
  }

  #[test]
  fn daily_log_counters_saturate() {
    let mut log = DailyLog::default();
    log.days.insert(
      "2026-10-01".to_owned(),
      DayTally {
        answered: u32::MAX,
        new: u32::MAX,
        duration_ms: u64::MAX,
      },
    );
    log.add("2026-10-01", true, 1);
    let t = log.on("2026-10-01");
    assert_eq!(
      (t.answered, t.new, t.duration_ms),
      (u32::MAX, u32::MAX, u64::MAX)
    );
  }

  #[test]
  fn intensity_needs_a_base() {
    // 从未作答：整列 0，视图据此画空状态。
    assert_eq!(intensity_levels([0, 0, 0], None), vec![0, 0, 0]);
    assert_eq!(intensity_levels([], None), Vec::<u8>::new());
    // goal = 0 等同于没有目标，不能除零，也不能把所有格子点亮。
    assert_eq!(intensity_levels([0, 0], Some(0)), vec![0, 0]);
    // goal = 0 要「回退到这一组的最大值」：用 `[0, 0]` 区分不出回退与直接全 0，
    // 这里让最大值非零 —— 最大那天满档，没做的那天仍是 0。
    assert_eq!(intensity_levels([3, 0], Some(0)), vec![5, 0]);
  }

  #[test]
  fn intensity_scales_against_goal() {
    // 目标 10 题：1 题也至少亮一档，10 题满档，超额封顶。
    assert_eq!(intensity_levels([1, 5, 10, 40], Some(10)), vec![1, 3, 5, 5]);
    // 没有目标时按这一组最大值归一：最大那天满档。
    assert_eq!(intensity_levels([0, 2, 8], None), vec![0, 2, 5]);
  }

  #[test]
  fn plan_days_left_and_legacy_json() {
    let plan: StudyPlan =
      serde_json::from_str(r#"{"target_date":"2026-10-11","target_count":500}"#).unwrap();
    assert_eq!(plan.bank, None);
    assert_eq!(plan.days_left("2026-10-01"), Some(10));
  }
}
