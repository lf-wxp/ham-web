//! `/eme` 计划器的纯计算部分：把「网格 + 频段 + 最低仰角 + 此刻」变成页面要展示的全部数字。
//!
//! 月球本身的算法与精度见 [`crate::moon`]；这里只做组装与输入校验，页面只管 DOM。
//! 放在 core 里是为了让「缺网格不硬算」「对方网格不合法」「最低仰角越界」这些分支有单测，
//! 而不是只靠 e2e 兜底。

use crate::grid::lat_lon_from_grid;
use crate::moon::{
  Observer, Phase, Topo, Window, echo_doppler_hz, phase, range_rate_km_s, topocentric, windows,
};

/// 表格与窗口扫描覆盖的总时长（小时）。
pub const SPAN_HOURS: f64 = 24.0;
/// 表格相邻两行的间隔（秒）：从「此刻」起每隔 1 小时一行，不是钟点整点。
pub const TABLE_STEP_S: f64 = 3600.0;
/// 窗口扫描的采样步长（秒）：窗口边界的分辨率就是它。
pub const WINDOW_STEP_S: f64 = 60.0;

/// 表格的一行：月球在地平线之上的一个时刻。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
  pub t: f64,
  pub topo: Topo,
  /// 自己的回波多普勒（Hz）。
  pub doppler_hz: f64,
}

/// 对方网格的处理结果。
#[derive(Debug, Clone, PartialEq)]
pub enum Mutual {
  /// 没填对方网格：不算共同窗口。
  NotRequested,
  /// 填了但不是合法网格。
  InvalidGrid,
  /// 两端同时在最低仰角之上的窗口。
  Windows(Vec<Window>),
}

/// 一次计算的全部结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
  pub now: f64,
  pub here: Topo,
  /// 此刻的站心距离变化率（km/s，正 = 远离）。
  pub rate: f64,
  pub phase: Phase,
  pub doppler_hz: f64,
  pub rows: Vec<Row>,
  pub mine: Vec<Window>,
  pub mutual: Mutual,
}

fn observer_of(grid: &str) -> Option<Observer> {
  lat_lon_from_grid(grid).map(|(lat, lon)| Observer::at(lat, lon))
}

/// 本台网格非法、最低仰角越界（须在 −90°–90°）或时刻不是有限数时为 `None`，不硬算。
/// `dx_grid` 留空表示不要共同窗口。
#[must_use]
pub fn plan(my_grid: &str, dx_grid: &str, band_mhz: u32, min_el: f64, now: f64) -> Option<Plan> {
  let my = observer_of(my_grid)?;
  if !now.is_finite() || !min_el.is_finite() || !(-90.0..=90.0).contains(&min_el) {
    return None;
  }
  let f_hz = f64::from(band_mhz) * 1.0e6;
  let rate = range_rate_km_s(now, my);
  let steps = (SPAN_HOURS * 3600.0 / TABLE_STEP_S) as usize;
  let rows = (0..=steps)
    .filter_map(|k| {
      let t = now + k as f64 * TABLE_STEP_S;
      let topo = topocentric(t, my);
      (topo.el_deg >= 0.0).then(|| Row {
        t,
        topo,
        doppler_hz: echo_doppler_hz(f_hz, range_rate_km_s(t, my)),
      })
    })
    .collect();
  let mutual = if dx_grid.trim().is_empty() {
    Mutual::NotRequested
  } else {
    observer_of(dx_grid).map_or(Mutual::InvalidGrid, |other| {
      Mutual::Windows(windows(
        my,
        Some(other),
        now,
        SPAN_HOURS,
        WINDOW_STEP_S,
        min_el,
      ))
    })
  };
  Some(Plan {
    now,
    here: topocentric(now, my),
    rate,
    phase: phase(now),
    doppler_hz: echo_doppler_hz(f_hz, rate),
    rows,
    mine: windows(my, None, now, SPAN_HOURS, WINDOW_STEP_S, min_el),
    mutual,
  })
}

/// 带符号的定点数，但「四舍五入后是零」时不带符号：`{:+.0}` 会把 −0.3 写成 `-0`。
#[must_use]
pub fn signed(v: f64, decimals: usize) -> String {
  let s = format!("{v:+.decimals$}");
  if s[1..].chars().all(|c| c == '0' || c == '.') {
    format!("{:.decimals$}", 0.0)
  } else {
    s
  }
}

/// Unix 秒 → UTC `MM-DD HH:MM`（向下取整到分钟）；非有限数给 `--`。
///
/// 不走浏览器的 `Date`：纯整数的民用历算法，原生环境里也能测。
#[must_use]
pub fn utc_stamp(unix_s: f64) -> String {
  if !unix_s.is_finite() {
    return "--".to_owned();
  }
  let secs = unix_s.floor() as i64;
  let (month, day) = month_day(secs.div_euclid(86_400));
  let sod = secs.rem_euclid(86_400);
  format!(
    "{month:02}-{day:02} {:02}:{:02}",
    sod / 3600,
    sod % 3600 / 60
  )
}

/// 自 1970-01-01 起的天数 → `(月, 日)`（Howard Hinnant 的 civil_from_days）。
fn month_day(days: i64) -> (i64, i64) {
  // 只要月和日，用不到纪元（年份）部分。
  let z = days + 719_468;
  let doe = z.rem_euclid(146_097);
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let day = doy - (153 * mp + 2) / 5 + 1;
  let month = if mp < 10 { mp + 3 } else { mp - 9 };
  (month, day)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 2026-11-10 00:00 UTC。
  const T0: f64 = 1_794_268_800.0;

  #[test]
  fn utc_stamp_formats_known_instants() {
    assert_eq!(utc_stamp(T0), "11-10 00:00");
    assert_eq!(
      utc_stamp(T0 + 8.0 * 3600.0 + 31.0 * 60.0 + 59.9),
      "11-10 08:31"
    );
    assert_eq!(utc_stamp(0.0), "01-01 00:00");
    // 闰日与跨年：2024-02-29 12:34:56 / 1999-12-31 23:59:59。
    assert_eq!(utc_stamp(1_709_210_096.0), "02-29 12:34");
    assert_eq!(utc_stamp(946_684_799.0), "12-31 23:59");
    // 1970 年之前向下取整，而不是向零取整。
    assert_eq!(utc_stamp(-1.0), "12-31 23:59");
    assert_eq!(utc_stamp(f64::NAN), "--");
  }

  #[test]
  fn signed_never_prints_negative_zero() {
    assert_eq!(signed(-0.3, 0), "0");
    assert_eq!(signed(0.4, 0), "0");
    assert_eq!(signed(-0.004, 2), "0.00");
    assert_eq!(signed(244.7, 0), "+245");
    assert_eq!(signed(-259.69, 0), "-260");
    assert_eq!(signed(-0.2547, 3), "-0.255");
  }

  #[test]
  fn plan_refuses_to_guess_when_the_inputs_are_unusable() {
    assert!(plan("", "", 144, 5.0, T0).is_none(), "缺本台网格");
    assert!(plan("ZZ99", "", 144, 5.0, T0).is_none(), "本台网格不合法");
    for bad in [-90.1, 90.1, f64::NAN, f64::INFINITY] {
      assert!(plan("OM89", "", 144, bad, T0).is_none(), "最低仰角 {bad}");
    }
    assert!(plan("OM89", "", 144, 5.0, f64::NAN).is_none());
    // 边界值本身合法。
    assert!(plan("OM89", "", 144, -90.0, T0).is_some());
    assert!(plan("OM89", "", 144, 90.0, T0).is_some());
  }

  /// 数值与 `e2e/tests/eme_planner.spec.ts` 里的 Horizons 参照一致（OM89，2026-11-10 00:00 UTC 起）。
  #[test]
  fn plan_lists_only_the_hours_when_the_moon_is_up() {
    let p = plan("OM89", "", 144, 5.0, T0).unwrap();
    assert_eq!(p.rows.len(), 10, "0–9 时月球在地平线之上");
    assert_eq!(p.rows[0].t, T0);
    assert!(p.rows.windows(2).all(|w| w[1].t - w[0].t == TABLE_STEP_S));
    assert!(p.rows.iter().all(|r| r.topo.el_deg >= 0.0));
    assert!((p.here.el_deg - 0.81).abs() < 0.25);
    assert_eq!(p.mutual, Mutual::NotRequested);
    // 同一时刻的多普勒与表格第一行一致；频段翻 9 倍，多普勒也翻 9 倍。
    assert!((p.doppler_hz - p.rows[0].doppler_hz).abs() < 1e-9);
    let p1296 = plan("OM89", "", 1296, 5.0, T0).unwrap();
    assert!((p1296.doppler_hz / p.doppler_hz - 9.0).abs() < 1e-9);
  }

  #[test]
  fn plan_windows_match_horizons_and_mutual_follows_the_dx_grid() {
    let p = plan("OM89", "", 144, 5.0, T0).unwrap();
    assert_eq!(p.mine.len(), 1);
    let w = p.mine[0];
    // Horizons 1 分钟序列：00:28–08:31（≥ 5°），容差 2 分钟。
    assert!((w.start_unix - (T0 + 28.0 * 60.0)).abs() <= 120.0);
    assert!((w.end_unix - (T0 + 511.0 * 60.0)).abs() <= 120.0);
    assert!(!w.clipped_start && !w.clipped_end);

    let both = plan("OM89", "QF56", 144, 5.0, T0).unwrap();
    let Mutual::Windows(list) = &both.mutual else {
      panic!("应有共同窗口：{:?}", both.mutual);
    };
    assert_eq!(list.len(), 1);
    assert!((list[0].start_unix - w.start_unix).abs() <= 120.0);

    assert_eq!(
      plan("OM89", "ZZ99", 144, 5.0, T0).unwrap().mutual,
      Mutual::InvalidGrid
    );
    assert_eq!(
      plan("OM89", "  ", 144, 5.0, T0).unwrap().mutual,
      Mutual::NotRequested
    );
    // 对方网格前后空白不影响。
    assert!(matches!(
      plan("OM89", " QF56 ", 144, 5.0, T0).unwrap().mutual,
      Mutual::Windows(_)
    ));
  }

  #[test]
  fn a_threshold_above_the_moons_peak_gives_no_windows() {
    let p = plan("OM89", "QF56", 144, 60.0, T0).unwrap();
    assert!(p.mine.is_empty());
    assert_eq!(p.mutual, Mutual::Windows(Vec::new()));
  }
}
