//! 竞赛配速：赛前目标与赛中「现在在哪、还差多少」。
//!
//! 全是算术：给定目标 QSO 数与计划时长，把时间轴按线性进度摊开，和实际完成数
//! 比较。**不预测传播、不建模休息规则** —— 它只回答「按这个速度，能不能在计划
//! 时间内做完」，与 [`crate::contest_analysis`] 的赛后统计互不替代。

/// 赛前目标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
  /// 目标 QSO 数。
  pub target_qsos: u32,
  /// 计划比赛时长（分钟）。
  pub planned_minutes: f64,
}

impl Goal {
  /// 目标是否可用（两项都大于 0）。
  #[must_use]
  pub fn is_set(&self) -> bool {
    self.target_qsos > 0 && self.planned_minutes > 0.0
  }
}

/// 配速报告。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pace {
  pub done: u32,
  /// 已用时间（分钟，负值按 0 计）。
  pub elapsed_min: f64,
  /// 按线性进度，此刻应完成的 QSO 数。
  pub expected: f64,
  /// `done - expected`：正数领先，负数落后。
  pub delta: f64,
  /// 实际速率（QSO/小时）；已用时间不足 [`MIN_RATE_ELAPSED_MIN`] 分钟时为 0。
  ///
  /// 开赛头一两分钟的分母太小（1 条 / 0.5 分钟 = 120 QSO/h），按它推算出来的
  /// `projected` / `eta_min` 全是噪声，所以那段时间一律按「速率未知」处理。
  pub current_rate: f64,
  /// 剩余时间内需要达到的速率；已完成目标或时间用完时为 `None`。
  pub required_rate: Option<f64>,
  /// 按当前速率达到目标还要多少分钟；速率为 0 时为 `None`。
  pub eta_min: Option<f64>,
  /// 按当前速率到计划结束时的预计 QSO 数。
  pub projected: f64,
  /// 计划时间是否已经用完。
  pub over_time: bool,
}

/// 起算「当前速率」的最小已用时长（分钟）。
///
/// 低于它时 `done / elapsed` 的分母太小：1 条 / 0.5 分钟 = 120 QSO/h，10 小时赛据此
/// 「预计」1200 条、`eta_min` 趋零，比不显示更误导。
const MIN_RATE_ELAPSED_MIN: f64 = 2.0;

/// 计算配速。`elapsed_min` 是开赛至今的分钟数。
#[must_use]
pub fn pace(done: u32, elapsed_min: f64, goal: &Goal) -> Pace {
  let elapsed = elapsed_min.max(0.0);
  let planned = goal.planned_minutes.max(0.0);
  let done_f = f64::from(done);
  let target = f64::from(goal.target_qsos);

  let hours = elapsed / 60.0;
  let current_rate = if elapsed >= MIN_RATE_ELAPSED_MIN {
    done_f / hours
  } else {
    0.0
  };
  let expected = if planned > 0.0 {
    target * (elapsed / planned).min(1.0)
  } else {
    0.0
  };
  let remaining_min = (planned - elapsed).max(0.0);
  let over_time = planned > 0.0 && remaining_min <= 0.0;
  let required_rate = if done_f >= target || over_time || remaining_min <= 0.0 {
    None
  } else {
    Some((target - done_f) / (remaining_min / 60.0))
  };
  let eta_min = if done_f >= target {
    Some(0.0)
  } else if current_rate > 0.0 {
    Some((target - done_f) / current_rate * 60.0)
  } else {
    None
  };
  let projected = if planned > 0.0 {
    current_rate * planned / 60.0
  } else {
    0.0
  };

  Pace {
    done,
    elapsed_min: elapsed,
    expected,
    delta: done_f - expected,
    current_rate,
    required_rate,
    eta_min,
    projected,
    over_time,
  }
}

/// 计划时长（分钟）回显成小时：最多两位小数并去掉尾零。
///
/// 只留一位会改写用户的输入（1.25 h 存成 75 min 再显示成「1.3」），而固定两位又会把
/// 2 小时显示成「2.00」。
#[must_use]
pub fn hours_label(minutes: u32) -> String {
  let text = format!("{:.2}", f64::from(minutes) / 60.0);
  text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// 分钟 → 输入框文本；0 表示「未填」，显示为空串而不是 `"0"`。
#[must_use]
pub fn hours_text(minutes: u32) -> String {
  if minutes == 0 {
    String::new()
  } else {
    hours_label(minutes)
  }
}

/// 输入框里的小时文本 → 计划分钟。
///
/// - 空串（含全空白）＝用户清空了这个字段，返回 `Some(0)`（0 表示「未设置」）；
/// - 非负的有限数字按四舍五入到分钟，过大的值饱和到 `u32::MAX`；
/// - 其它（非数字、负数、`NaN` / `inf`）返回 `None`：**调用方不应写回**，否则用户敲到一半的
///   中间态会被改写成 0，字段当场被清空。
#[must_use]
pub fn parse_hours(text: &str) -> Option<u32> {
  let text = text.trim();
  if text.is_empty() {
    return Some(0);
  }
  let hours = text.parse::<f64>().ok()?;
  if !hours.is_finite() || hours < 0.0 {
    return None;
  }
  let minutes = (hours * 60.0).round();
  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  Some(minutes.min(f64::from(u32::MAX)) as u32)
}

/// 输入框里的目标 QSO 数 → 数量。
///
/// 语义与 [`parse_hours`] 一致：空串＝清除（`Some(0)`）；非负整数照收；`1.5`、负号、
/// 超出 `u32` 的数返回 `None`，调用方不写回，免得把非法输入静默存成 0 而清空目标。
#[must_use]
pub fn parse_target_qsos(text: &str) -> Option<u32> {
  let text = text.trim();
  if text.is_empty() {
    return Some(0);
  }
  text.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hours_round_trip_for_every_whole_minute() {
    // 两位小数的最大误差是 0.005 h = 0.3 min，一定能四舍五入回原来的分钟数。
    for minutes in 1..=3000u32 {
      assert_eq!(
        parse_hours(&hours_label(minutes)),
        Some(minutes),
        "{minutes} min → {:?}",
        hours_label(minutes)
      );
    }
  }

  #[test]
  fn hours_label_keeps_what_the_user_typed() {
    assert_eq!(hours_label(60), "1");
    assert_eq!(hours_label(75), "1.25");
    assert_eq!(hours_label(90), "1.5");
    assert_eq!(hours_label(120), "2");
    assert_eq!(hours_label(6000), "100");
    assert_eq!(hours_label(0), "0");
    // 0 在输入框里是「未填」，不是字面的 "0"。
    assert_eq!(hours_text(0), "");
    assert_eq!(hours_text(75), "1.25");
  }

  #[test]
  fn parse_hours_clears_on_empty_and_ignores_garbage() {
    assert_eq!(parse_hours(""), Some(0));
    assert_eq!(parse_hours("   "), Some(0));
    assert_eq!(parse_hours(" 1.5 "), Some(90));
    assert_eq!(parse_hours("0"), Some(0));
    // 敲到一半的 `1.` 能解析（按 1 小时），小数点留在草稿里由调用方保管。
    assert_eq!(parse_hours("1."), Some(60));
    for bad in ["abc", "-1", "-0.5", "1,5", "NaN", "inf", "-inf", "1e", "--"] {
      assert_eq!(parse_hours(bad), None, "{bad:?} 不该写回");
    }
    // 过大的值饱和而不是回绕。
    assert_eq!(parse_hours("1e12"), Some(u32::MAX));
  }

  #[test]
  fn parse_target_qsos_only_accepts_whole_non_negative_numbers() {
    assert_eq!(parse_target_qsos(""), Some(0));
    assert_eq!(parse_target_qsos(" 120 "), Some(120));
    assert_eq!(parse_target_qsos("007"), Some(7));
    for bad in ["1.5", "-3", "abc", "4294967296", "1e3", "+"] {
      assert_eq!(parse_target_qsos(bad), None, "{bad:?} 不该写回");
    }
  }

  fn goal(target_qsos: u32, planned_hours: f64) -> Goal {
    Goal {
      target_qsos,
      planned_minutes: planned_hours * 60.0,
    }
  }

  #[test]
  fn the_first_two_minutes_do_not_project_absurd_rates() {
    // 开赛 30 秒做了 1 条：速率口径先按未知处理，避免「预计 1200 条」。
    let p = pace(1, 0.5, &goal(100, 10.0));
    assert_eq!(p.current_rate, 0.0);
    assert_eq!(p.eta_min, None);
    assert_eq!(p.projected, 0.0);
    // 满 2 分钟后恢复正常口径：1 条 / 2 分钟 = 30 QSO/h。
    let q = pace(1, 2.0, &goal(100, 10.0));
    assert!((q.current_rate - 30.0).abs() < 1e-9, "{}", q.current_rate);
    assert!(q.projected > 0.0);
  }

  #[test]
  fn exactly_on_pace_has_zero_delta() {
    let p = pace(50, 300.0, &goal(100, 10.0));
    assert!((p.expected - 50.0).abs() < 1e-9);
    assert!(p.delta.abs() < 1e-9);
    assert!((p.current_rate - 10.0).abs() < 1e-9);
    assert!((p.projected - 100.0).abs() < 1e-9);
    assert!(p.required_rate.is_some_and(|r| (r - 10.0).abs() < 1e-9));
  }

  #[test]
  fn behind_pace_needs_a_higher_rate() {
    let p = pace(20, 300.0, &goal(100, 10.0));
    assert!(p.delta < 0.0);
    let required = p.required_rate.expect("还能追上");
    let current = p.current_rate;
    assert!(required > current);
    // 20 条用了 5 小时 = 4 条/小时；剩 5 小时要 80 条 = 16 条/小时。
    assert!((current - 4.0).abs() < 1e-9);
    assert!((required - 16.0).abs() < 1e-9);
  }

  #[test]
  fn reaching_the_target_clears_the_requirement() {
    let p = pace(120, 300.0, &goal(100, 10.0));
    assert_eq!(p.required_rate, None);
    assert_eq!(p.eta_min, Some(0.0));
    assert!(p.delta > 0.0);
  }

  #[test]
  fn no_elapsed_time_has_no_rate_and_no_eta() {
    let p = pace(3, 0.0, &goal(100, 10.0));
    assert_eq!(p.current_rate, 0.0);
    assert_eq!(p.eta_min, None);
    assert_eq!(p.expected, 0.0);
    assert!((p.delta - 3.0).abs() < 1e-9);
    // 剩余 97 条要摊在 10 小时里。
    assert!((p.required_rate.expect("还能追上") - 9.7).abs() < 1e-9);
  }

  #[test]
  fn running_out_of_time_reports_over_time() {
    let p = pace(40, 660.0, &goal(100, 10.0));
    assert!(p.over_time);
    assert_eq!(p.required_rate, None);
    // 进度项在时间用完后封顶在目标值。
    assert!((p.expected - 100.0).abs() < 1e-9);
    assert!(p.delta < 0.0);
  }

  #[test]
  fn unset_goal_is_harmless() {
    let unset = Goal {
      target_qsos: 0,
      planned_minutes: 0.0,
    };
    let p = pace(0, 30.0, &unset);
    assert_eq!(p.expected, 0.0);
    assert_eq!(p.projected, 0.0);
    assert_eq!(p.required_rate, None);
    assert!(!unset.is_set());
  }

  #[test]
  fn negative_elapsed_is_clamped() {
    let p = pace(10, -5.0, &goal(100, 10.0));
    assert_eq!(p.elapsed_min, 0.0);
    assert_eq!(p.current_rate, 0.0);
  }
}
