//! 考试倒计时的分级与预警：纯计算，不依赖浏览器。
//!
//! 为什么单独成一个模块：以前考试页只有「最后 60 秒数字变红」一级，三处读数（顶部元信息、
//! Boss 舞台、将来的底栏）各抄一份阈值；超时自动交卷时更是没有任何提示，用户没有心理准备就
//! 被交卷。这里把「现在处于哪一档」「这一秒有没有刚好跨过一档」收成两个可测的函数，视图只管
//! 渲染与弹提示。

/// 倒计时所处的档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockTone {
  /// 时间充裕（或不限时）。
  Calm,
  /// 进入提醒区：该检查未答题了。
  Warn,
  /// 最后冲刺：马上自动交卷。
  Danger,
}

const MINUTE_MS: i64 = 60_000;

/// 提醒线：剩余不足 5 分钟，但最多取总时长的 1/4。
///
/// 为什么要封顶到 1/4：自定义组卷可以只限时几分钟，固定的「5 分钟」会让它一开考就处于提醒区。
#[must_use]
pub const fn warn_at_ms(total_ms: i64) -> i64 {
  let quarter = total_ms / 4;
  if quarter < 5 * MINUTE_MS {
    quarter
  } else {
    5 * MINUTE_MS
  }
}

/// 冲刺线：剩余不足 1 分钟，但最多取总时长的 1/10。
#[must_use]
pub const fn danger_at_ms(total_ms: i64) -> i64 {
  let tenth = total_ms / 10;
  if tenth < MINUTE_MS { tenth } else { MINUTE_MS }
}

/// 当前档位。`total_ms <= 0` 表示不限时，永远是 [`ClockTone::Calm`]。
#[must_use]
pub const fn clock_tone(remaining_ms: i64, total_ms: i64) -> ClockTone {
  if total_ms <= 0 {
    return ClockTone::Calm;
  }
  if remaining_ms <= danger_at_ms(total_ms) {
    ClockTone::Danger
  } else if remaining_ms <= warn_at_ms(total_ms) {
    ClockTone::Warn
  } else {
    ClockTone::Calm
  }
}

/// 从 `prev_ms` 走到 `now_ms` 的这一步，有没有**刚好跨过**某条线。
///
/// 返回跨过的最严重的一档（一步同时跨过两条线时，比如后台标签页被冻结后恢复，只报
/// [`ClockTone::Danger`]，不连弹两条）。没跨过、不限时、或时间在往回走都返回 `None` ——
/// 每条线一场考试只提醒一次，由「跨过」而不是「处于」来触发。
#[must_use]
pub const fn crossed_line(prev_ms: i64, now_ms: i64, total_ms: i64) -> Option<ClockTone> {
  if total_ms <= 0 || now_ms >= prev_ms {
    return None;
  }
  let (warn, danger) = (warn_at_ms(total_ms), danger_at_ms(total_ms));
  if prev_ms > danger && now_ms <= danger {
    Some(ClockTone::Danger)
  } else if prev_ms > warn && now_ms <= warn {
    Some(ClockTone::Warn)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const MIN: i64 = MINUTE_MS;

  #[test]
  fn lines_scale_down_for_short_papers() {
    // 标准 40 分钟：5 分钟 / 1 分钟
    assert_eq!(warn_at_ms(40 * MIN), 5 * MIN);
    assert_eq!(danger_at_ms(40 * MIN), MIN);
    // 8 分钟的自定义卷：2 分钟 / 48 秒，不会一开考就处于提醒区
    assert_eq!(warn_at_ms(8 * MIN), 2 * MIN);
    assert_eq!(danger_at_ms(8 * MIN), 48_000);
    assert_eq!(clock_tone(8 * MIN, 8 * MIN), ClockTone::Calm);
  }

  #[test]
  fn tone_follows_the_lines_and_untimed_is_always_calm() {
    let total = 40 * MIN;
    assert_eq!(clock_tone(30 * MIN, total), ClockTone::Calm);
    assert_eq!(
      clock_tone(5 * MIN, total),
      ClockTone::Warn,
      "恰好在线上算进入"
    );
    assert_eq!(clock_tone(MIN + 1, total), ClockTone::Warn);
    assert_eq!(clock_tone(MIN, total), ClockTone::Danger);
    assert_eq!(clock_tone(0, total), ClockTone::Danger);
    // 不限时（自定义组卷 minutes == 0）：没有倒计时可言
    assert_eq!(clock_tone(0, 0), ClockTone::Calm);
  }

  #[test]
  fn crossing_fires_once_per_line() {
    let total = 40 * MIN;
    // 每秒一步：只有刚好越线的那一步触发
    assert_eq!(
      crossed_line(5 * MIN + 1000, 5 * MIN, total),
      Some(ClockTone::Warn)
    );
    assert_eq!(
      crossed_line(5 * MIN, 5 * MIN - 1000, total),
      None,
      "已经在线内，不再提醒"
    );
    assert_eq!(
      crossed_line(MIN + 1000, MIN, total),
      Some(ClockTone::Danger)
    );
    assert_eq!(crossed_line(MIN, MIN - 1000, total), None);
    assert_eq!(crossed_line(20 * MIN, 20 * MIN - 1000, total), None);
  }

  #[test]
  fn a_frozen_tab_reports_only_the_most_severe_line() {
    let total = 40 * MIN;
    // 后台标签页被冻结，恢复时一步跨过两条线：只报 Danger，不连弹两条
    assert_eq!(
      crossed_line(10 * MIN, 30_000, total),
      Some(ClockTone::Danger)
    );
  }

  #[test]
  fn untimed_or_backwards_clock_never_fires() {
    assert_eq!(crossed_line(10, 0, 0), None, "不限时");
    // 首次 tick 前 remaining 是 0，随后跳到真实剩余：不能当成「越线」
    assert_eq!(crossed_line(0, 40 * MIN, 40 * MIN), None);
  }
}
