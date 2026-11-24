//! 数字模式时隙钟：FT8 / FT4 / WSPR 都按 UTC 对齐的固定周期收发，这里算「现在在第几个时隙、
//! 这个时隙从哪一刻起、离下一个还有多久」。
//!
//! 只算时间，不碰电台、不发 PTT：它回答的是「我该在哪一刻按下播放 / 开始发射」。
//! 所有周期都是整数毫秒、且能整除一分钟的整数倍，所以时隙边界与 Unix 纪元（UTC 整分钟）对齐 ——
//! 不需要时区、不需要闰秒表（Unix 时间本来就不计闰秒，与 WSJT-X 取同一口径）。
//!
//! | 模式 | 周期 | 起点 |
//! | --- | --- | --- |
//! | FT8 | 15 s | 每分钟的 :00 / :15 / :30 / :45 |
//! | FT4 | 7.5 s | :00 / :07.5 / :15 / :22.5 … |
//! | WSPR | 120 s | 每个偶数分钟；发射从周期的第 1 秒开始（[`SlotSpec::tx_offset_ms`]） |
//!
//! 精度边界：时间取自浏览器（`Date.now()`），即本机系统时钟。这些模式都要求时钟误差在 1 秒左右以内，
//! 页面会提示用户先对时；本模块不知道、也不猜系统时钟准不准。

/// 一种按固定周期对齐 UTC 的模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSpec {
  /// 模式键（`"ft8"` / `"ft4"` / `"wspr"`），界面按它取文案。
  pub key: &'static str,
  /// 周期（毫秒）。
  pub period_ms: u64,
  /// 发射从周期起点之后多久开始（毫秒）：WSPR 是 1 秒，FT8 / FT4 从起点开始（0）。
  pub tx_offset_ms: u64,
}

/// FT8：15 秒一个时隙。
pub const FT8: SlotSpec = SlotSpec {
  key: "ft8",
  period_ms: 15_000,
  tx_offset_ms: 0,
};

/// FT4：7.5 秒一个时隙。
pub const FT4: SlotSpec = SlotSpec {
  key: "ft4",
  period_ms: 7_500,
  tx_offset_ms: 0,
};

/// WSPR：每 2 分钟一个周期，偶数分钟起算，发射从第 1 秒开始。
pub const WSPR: SlotSpec = SlotSpec {
  key: "wspr",
  period_ms: 120_000,
  tx_offset_ms: 1_000,
};

/// 此刻在某个模式时隙里的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
  /// 时隙序号：自 Unix 纪元起的第几个周期。奇偶用 [`Position::is_even`]。
  pub index: u64,
  /// 本时隙起点（自 Unix 纪元的毫秒）。
  pub start_ms: u64,
  /// 已过去多久（毫秒，`0 ≤ elapsed < period`）。
  pub elapsed_ms: u64,
  /// 离下一个时隙还有多久（毫秒，`0 < remaining ≤ period`；恰在起点时等于整个周期）。
  pub remaining_ms: u64,
  /// 周期（毫秒），方便界面算进度。
  pub period_ms: u64,
}

/// 现在（自 Unix 纪元的毫秒，`Date.now()` 的值）在 `spec` 的哪个时隙里。
///
/// 非有限或为负的时间返回 `None`（时钟读不出来时界面不画，而不是显示一个编出来的数）。
#[must_use]
pub fn position(spec: &SlotSpec, now_ms: f64) -> Option<Position> {
  if !now_ms.is_finite() || now_ms < 0.0 || spec.period_ms == 0 {
    return None;
  }
  // 毫秒取整：`Date.now()` 本来就是整数毫秒；小数部分（性能计时器之类）向下取。
  let now = now_ms as u64;
  let index = now / spec.period_ms;
  let elapsed_ms = now % spec.period_ms;
  Some(Position {
    index,
    start_ms: index * spec.period_ms,
    elapsed_ms,
    remaining_ms: spec.period_ms - elapsed_ms,
    period_ms: spec.period_ms,
  })
}

impl Position {
  /// 偶数时隙：FT8 的 :00 / :30，FT4 的 :00 / :15 / :30 / :45。
  ///
  /// 通联里常见的约定：CQ 的一方在偶数时隙发、回呼的一方在奇数时隙发，两边错开才能互相听见。
  #[must_use]
  pub const fn is_even(&self) -> bool {
    self.index.is_multiple_of(2)
  }

  /// 剩余时间，以 0.1 秒为单位**向上取整**：显示 `0.0` 只会出现在「恰好已经是下一个时隙」的瞬间，
  /// 此时 `remaining_ms` 已经回到整个周期，所以用户永远看不到「0.0 秒」还没开始。
  #[must_use]
  pub const fn remaining_tenths(&self) -> u64 {
    self.remaining_ms.div_ceil(100)
  }

  /// 时隙进度（0 ≤ 值 < 1）。
  #[must_use]
  pub fn progress(&self) -> f64 {
    self.elapsed_ms as f64 / self.period_ms as f64
  }

  /// 本时隙起点的 UTC 时刻：`(时, 分, 秒, 十分之一秒)`。
  ///
  /// 直接由纪元毫秒取模算出（UTC 一天恒为 86 400 秒，Unix 时间不计闰秒），不借助浏览器的日期对象。
  #[must_use]
  pub const fn start_clock(&self) -> (u32, u32, u32, u32) {
    let in_day_ms = self.start_ms % 86_400_000;
    let secs = (in_day_ms / 1000) as u32;
    (
      secs / 3600,
      secs % 3600 / 60,
      secs % 60,
      (in_day_ms % 1000 / 100) as u32,
    )
  }
}

/// 现在的 UTC 时刻：`(时, 分, 秒)`。
#[must_use]
pub fn utc_clock(now_ms: f64) -> Option<(u32, u32, u32)> {
  if !now_ms.is_finite() || now_ms < 0.0 {
    return None;
  }
  let secs = (now_ms as u64 % 86_400_000 / 1000) as u32;
  Some((secs / 3600, secs % 3600 / 60, secs % 60))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 2026-11-25T10:00:00Z 的毫秒（独立算出：Python `datetime(2026, 11, 25, 10, tzinfo=utc).timestamp()` = 1 795 600 800 s）。
  /// 它既是 UTC 整点也是偶数分钟：下面的断言只依赖这两个事实。
  const TEN_OCLOCK: f64 = 1_795_600_800_000.0;

  #[test]
  fn the_fixture_is_a_utc_hour_boundary() {
    assert_eq!(TEN_OCLOCK as u64 % 3_600_000, 0);
    assert_eq!(TEN_OCLOCK as u64 / 1000, 1_795_600_800);
    assert_eq!(utc_clock(TEN_OCLOCK), Some((10, 0, 0)));
  }

  /// FT8：:00 / :15 / :30 / :45 起算；边界前 1 ms 还在上一个时隙，边界上就是新时隙。
  #[test]
  fn ft8_slots_start_on_quarter_minutes() {
    let at = |ms: f64| position(&FT8, TEN_OCLOCK + ms).unwrap();
    let p = at(7_000.0);
    assert_eq!((p.elapsed_ms, p.remaining_ms), (7_000, 8_000));
    assert_eq!(p.start_clock(), (10, 0, 0, 0));

    let before = at(14_999.0);
    let on = at(15_000.0);
    assert_eq!(before.remaining_ms, 1);
    assert_eq!(on.elapsed_ms, 0);
    assert_eq!(
      on.remaining_ms, 15_000,
      "恰在起点时离下一个还有整个周期，不是 0"
    );
    assert_eq!(on.index, before.index + 1);
    assert_eq!(on.start_clock(), (10, 0, 15, 0));
    assert_eq!(at(45_000.0).start_clock(), (10, 0, 45, 0));
    assert_eq!(at(60_000.0).start_clock(), (10, 1, 0, 0));
  }

  /// FT4：7.5 秒 —— 起点落在 :07.5、:22.5 这样的半秒上（`start_clock` 的十分之一秒要对）。
  #[test]
  fn ft4_slots_start_on_half_seconds() {
    let at = |ms: f64| position(&FT4, TEN_OCLOCK + ms).unwrap();
    assert_eq!(at(7_499.0).start_clock(), (10, 0, 0, 0));
    assert_eq!(at(7_500.0).start_clock(), (10, 0, 7, 5));
    assert_eq!(at(15_000.0).start_clock(), (10, 0, 15, 0));
    assert_eq!(at(22_500.0).start_clock(), (10, 0, 22, 5));
    // 一个 15 秒的 FT8 时隙里恰有两个 FT4 时隙。
    assert_eq!(at(15_000.0).index, at(0.0).index + 2);
  }

  /// WSPR：每个偶数分钟起算 120 秒一个周期；发射偏移是 1 秒（只是数据，位置本身从周期起点算）。
  #[test]
  fn wspr_cycles_start_on_even_minutes() {
    assert_eq!(TEN_OCLOCK as u64 / 60_000 % 2, 0, "夹具：10:00 是偶数分钟");
    let at = |ms: f64| position(&WSPR, TEN_OCLOCK + ms).unwrap();
    assert_eq!(at(0.0).start_clock(), (10, 0, 0, 0));
    assert_eq!(at(119_999.0).start_clock(), (10, 0, 0, 0));
    assert_eq!(at(120_000.0).start_clock(), (10, 2, 0, 0));
    assert_eq!(at(61_000.0).remaining_ms, 59_000);
    assert_eq!(WSPR.tx_offset_ms, 1_000);
    assert_eq!((FT8.tx_offset_ms, FT4.tx_offset_ms), (0, 0));
  }

  /// 奇偶：FT8 的偶数时隙是 :00 / :30，奇数是 :15 / :45；相邻时隙奇偶相反。
  #[test]
  fn parity_alternates_and_matches_the_clock() {
    for (secs, even) in [
      (0.0, true),
      (15.0, false),
      (30.0, true),
      (45.0, false),
      (60.0, true),
    ] {
      let p = position(&FT8, TEN_OCLOCK + secs * 1000.0).unwrap();
      assert_eq!(p.is_even(), even, "{secs} s");
    }
    for (secs, even) in [(0.0, true), (7.5, false), (15.0, true), (22.5, false)] {
      let p = position(&FT4, TEN_OCLOCK + secs * 1000.0).unwrap();
      assert_eq!(p.is_even(), even, "FT4 {secs} s");
    }
  }

  /// 剩余时间按 0.1 秒向上取整：差 1 ms 到边界显示 0.1 而不是 0.0；整个周期则是 15.0。
  #[test]
  fn remaining_time_rounds_up_so_it_never_shows_zero_before_the_slot_starts() {
    let tenths = |ms: f64| position(&FT8, TEN_OCLOCK + ms).unwrap().remaining_tenths();
    assert_eq!(tenths(14_999.0), 1);
    assert_eq!(tenths(14_900.0), 1);
    assert_eq!(tenths(14_899.0), 2);
    assert_eq!(tenths(7_000.0), 80);
    assert_eq!(tenths(15_000.0), 150);
    assert_eq!(tenths(0.0), 150);
  }

  #[test]
  fn progress_runs_from_zero_up_to_but_not_including_one() {
    let p = |ms: f64| position(&FT8, TEN_OCLOCK + ms).unwrap().progress();
    assert_eq!(p(0.0), 0.0);
    assert!((p(7_500.0) - 0.5).abs() < 1e-12);
    assert!(p(14_999.0) < 1.0);
    assert_eq!(p(15_000.0), 0.0);
  }

  #[test]
  fn unreadable_clocks_give_nothing_instead_of_an_invented_value() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
      assert_eq!(position(&FT8, bad), None, "{bad}");
      assert_eq!(utc_clock(bad), None, "{bad}");
    }
    let zero = SlotSpec {
      key: "x",
      period_ms: 0,
      tx_offset_ms: 0,
    };
    assert_eq!(position(&zero, 1000.0), None);
    // 很大的时间也不 panic（2^53 ms 是 f64 能精确表示的整数上限）。
    assert!(position(&FT8, 9_007_199_254_740_992.0).is_some());
  }

  /// 午夜前后：一天恒为 86 400 秒，`start_clock` 在 23:59:45 → 00:00:00 处回绕。
  #[test]
  fn the_clock_wraps_at_utc_midnight() {
    let midnight = (TEN_OCLOCK as u64 / 86_400_000 + 1) as f64 * 86_400_000.0;
    let p = position(&FT8, midnight - 1.0).unwrap();
    assert_eq!(p.start_clock(), (23, 59, 45, 0));
    let p = position(&FT8, midnight).unwrap();
    assert_eq!(p.start_clock(), (0, 0, 0, 0));
    assert_eq!(utc_clock(midnight - 1.0), Some((23, 59, 59)));
  }
}
