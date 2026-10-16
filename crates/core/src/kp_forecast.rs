//! Kp 三日预报：数据校验、按「日 × 3 小时时段」整理、预测期最高值与 NOAA G 级。
//!
//! 数据来自 NOAA SWPC 的 `noaa-planetary-k-index-forecast.json`：约十天、每 3 小时一条，
//! 每条标明是 `observed`（实测）、`estimated`（当天尚未定稿的估计）还是 `predicted`（预报）。
//! 服务端负责取数与校验（[`clean`]），页面负责展示；这里是两边共用的纯函数。
//!
//! **时间一律是 UTC，且是时段的起点**：`2026-10-15T21:00:00` 就是 NOAA 文字版里的「21-00UT」。
//!
//! **G 级是这里按 NOAA 标度算的**，不是上游给的：上游只在已观测的行里带 `noaa_scale`，
//! 预报行里没有。规则见 [`g_scale`]。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::study_plan::{day_number, format_day};

/// 每天的 3 小时时段数。
pub const SLOTS_PER_DAY: usize = 8;

/// 时段标签（UTC，起点–终点；与 NOAA 文字版的写法一致，末段为 `21–00`）。
pub const SLOT_LABELS: [&str; SLOTS_PER_DAY] = [
  "00–03", "03–06", "06–09", "09–12", "12–15", "15–18", "18–21", "21–00",
];

/// 这条 Kp 是怎么来的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KpKind {
  /// 已实测定稿。
  Observed,
  /// 当天已过去但尚未定稿的估计值。
  Estimated,
  /// 预报。
  Predicted,
}

impl KpKind {
  /// 上游 `observed` 字段的取值；不认识的返回 `None`。
  #[must_use]
  pub fn from_upstream(s: &str) -> Option<Self> {
    match s {
      "observed" => Some(Self::Observed),
      "estimated" => Some(Self::Estimated),
      "predicted" => Some(Self::Predicted),
      _ => None,
    }
  }
}

/// 一个 3 小时时段的 Kp。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KpPoint {
  /// 时段起点，`YYYY-MM-DDTHH:00:00`（UTC）。
  pub time: String,
  pub kp: f64,
  pub kind: KpKind,
}

/// 服务端 `/api/kp-forecast` 的响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KpForecast {
  pub source: String,
  /// 最近一条**实测**的时段起点；一条实测都没有时为 `None`。
  pub updated: Option<String>,
  /// 按时间升序、每个时段至多一条。
  pub points: Vec<KpPoint>,
}

/// 把 `time` 拆成 `(日期, 时段序号 0..8)`，同时检查它是不是一个合法的时段起点。
///
/// 日期必须真实存在（`2026-02-30` 不行：共用的 `day_number` 只检查「日在 1–31」，
/// 所以转成天数再转回来必须一字不差）；时必须是 3 的倍数，分与秒必须是 0。
fn split_time(time: &str) -> Option<(String, usize)> {
  let (date, clock) = time.split_once('T')?;
  if format_day(day_number(date)?) != date {
    return None;
  }
  let mut parts = clock.split(':');
  let hour: usize = parts.next()?.parse().ok()?;
  let minute: u32 = parts.next()?.parse().ok()?;
  let second_ok = parts.next().is_none_or(|s| s.parse::<u32>() == Ok(0));
  if parts.next().is_some() || minute != 0 || !second_ok || hour >= 24 || !hour.is_multiple_of(3) {
    return None;
  }
  Some((date.to_owned(), hour / 3))
}

fn valid_kp(kp: f64) -> bool {
  kp.is_finite() && (0.0..=9.0).contains(&kp)
}

/// 校验、规范化并排序：丢掉 Kp 不在 0–9、时间不是合法时段起点的行；同一时段多条时后面的覆盖前面的。
///
/// 返回（干净的序列，被丢掉的行数）。时间会被规范成 `YYYY-MM-DDTHH:00:00`，
/// 所以 `…T03:00` 与 `…T03:00:00` 是同一个时段，不会各占一格。
#[must_use]
pub fn clean(points: Vec<KpPoint>) -> (Vec<KpPoint>, usize) {
  let mut by_time: BTreeMap<String, KpPoint> = BTreeMap::new();
  let mut dropped = 0;
  for p in points {
    let Some((date, slot)) = split_time(&p.time).filter(|_| valid_kp(p.kp)) else {
      dropped += 1;
      continue;
    };
    let time = format!("{date}T{:02}:00:00", slot * 3);
    if by_time
      .insert(time.clone(), KpPoint { time, ..p })
      .is_some()
    {
      dropped += 1;
    }
  }
  (by_time.into_values().collect(), dropped)
}

/// NOAA 地磁暴 G 级（1–5）；低于 G1 返回 `None`。
///
/// Kp 以三分之一为单位（`5-` = 4.67，`5` = 5.00，`5+` = 5.33），NOAA 标度按整数 Kp 分级：
/// G1 = 5，G2 = 6，G3 = 7，G4 = 8（**含 9-**），G5 = 9。取整数 Kp 用四舍五入，
/// 所以 `4.67`（5-）算 G1；唯一的例外是 `8.67`（9-）：NOAA 明文把它归到 G4，G5 要整 9。
///
/// 依据：上游数据里 6.67（= 7-）→ G3，向下取整会得到 G2，所以是四舍五入；5.33 → G1、7.0 → G3 也对得上；
/// NOAA 标度页的「Kp = 8, including a 9-」。**没有直接验证的一点**：上游数据里没有出现过 4.33–4.99 的值，
/// 所以 4.67（5-）算 G1 是按同一个取整规则推出来的，不是看到上游这么标。
#[must_use]
pub fn g_scale(kp: f64) -> Option<u8> {
  if !valid_kp(kp) {
    return None;
  }
  let whole = kp.round();
  // 整数 Kp 是 0..=9，转成 u8 不会截断。
  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  let whole = whole as u8;
  let g = whole.saturating_sub(4);
  let g = if kp < 9.0 - 1e-9 { g.min(4) } else { g };
  (g >= 1).then_some(g)
}

/// 一格的强弱分档（用来着色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KpLevel {
  /// 整数 Kp ≤ 3（含 3+ = 3.33，不含 4- = 3.67）。
  Quiet,
  /// 整数 Kp = 4（3.67 到 4.33）：地磁活跃，还不是磁暴。
  Active,
  /// G1–G5。
  Storm(u8),
}

#[must_use]
pub fn level(kp: f64) -> KpLevel {
  match g_scale(kp) {
    Some(g) => KpLevel::Storm(g),
    None if valid_kp(kp) && kp.round() >= 4.0 => KpLevel::Active,
    None => KpLevel::Quiet,
  }
}

/// 一格。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
  pub kp: f64,
  pub kind: KpKind,
}

/// 某一天的 8 个时段（缺的是 `None`）。
#[derive(Debug, Clone, PartialEq)]
pub struct DayRow {
  /// `YYYY-MM-DD`（UTC）。
  pub date: String,
  pub slots: [Option<Slot>; SLOTS_PER_DAY],
}

/// 预测期里最高的一格。
#[derive(Debug, Clone, PartialEq)]
pub struct Peak {
  pub kp: f64,
  pub date: String,
  /// 0..8，对应 [`SLOT_LABELS`]。
  pub slot: usize,
  pub g: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outlook {
  /// 展示的几天，升序；没有数据的天不出现。
  pub days: Vec<DayRow>,
  /// 预测期（见 [`outlook`]）里非实测的最高一格，并列取最早；没有非实测的格就是 `None`。
  pub peak: Option<Peak>,
}

/// 整理成页面要的样子。没有任何有效数据时返回 `None`。
///
/// 「今天」`D0` = 第一个**不是实测**的时段所在的那一天（没有就取最后一天）。展示 `D0-1 ..= D0+2`
/// 共四天，也就是昨天加上 NOAA 的三日预报窗口。预测期最高值只看 `D0 ..= D0+2` 里**非实测**的格：
/// 已经测到的高值是历史，不是预报。
///
/// 上游在预测期末尾会多给一条下一天 `00:00` 的预报，落在窗口之外，这里不展示（NOAA 的文字版也不列）。
/// 实时页面不得将所有时段都已过期的缓存当成当前预报。
#[must_use]
pub fn live_outlook(points: &[KpPoint], utc_minutes: i64) -> Option<Outlook> {
  let (cleaned, _) = clean(points.to_vec());
  let latest = cleaned.last()?;
  let (date, slot) = split_time(&latest.time)?;
  let until = day_number(&date)?
    .checked_mul(1440)?
    .checked_add(((slot + 1) * 180) as i64)?;
  if utc_minutes >= until {
    return None;
  }
  outlook(&cleaned)
}

#[must_use]
pub fn outlook(points: &[KpPoint]) -> Option<Outlook> {
  let (pts, _) = clean(points.to_vec());
  let date_of = |p: &KpPoint| p.time[..10].to_owned();
  let d0 = pts
    .iter()
    .find(|p| p.kind != KpKind::Observed)
    .or(pts.last())
    .map(date_of)?;
  let d0n = day_number(&d0)?;
  let window: Vec<String> = (-1..=2).map(|i| format_day(d0n + i)).collect();

  let mut days = Vec::new();
  for date in &window {
    let mut slots = [None; SLOTS_PER_DAY];
    for p in pts.iter().filter(|p| p.time.starts_with(date.as_str())) {
      if let Some((_, slot)) = split_time(&p.time) {
        slots[slot] = Some(Slot {
          kp: p.kp,
          kind: p.kind,
        });
      }
    }
    if slots.iter().any(Option::is_some) {
      days.push(DayRow {
        date: date.clone(),
        slots,
      });
    }
  }

  // 预测期：D0..=D0+2；`pts` 已按时间升序，`>` 严格大于保证并列时留下最早的。
  let mut peak: Option<Peak> = None;
  for p in pts.iter().filter(|p| p.kind != KpKind::Observed) {
    let date = date_of(p);
    if !window[1..].contains(&date) {
      continue;
    }
    if peak.as_ref().is_none_or(|b| p.kp > b.kp) {
      let slot = split_time(&p.time).map_or(0, |(_, s)| s);
      peak = Some(Peak {
        kp: p.kp,
        date,
        slot,
        g: g_scale(p.kp),
      });
    }
  }

  (!days.is_empty()).then_some(Outlook { days, peak })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn p(time: &str, kp: f64, kind: KpKind) -> KpPoint {
    KpPoint {
      time: time.to_owned(),
      kp,
      kind,
    }
  }

  use KpKind::{Estimated as E, Observed as O, Predicted as P};

  /// 2026-10-15 取回的 SWPC 原始数据（`noaa-planetary-k-index-forecast.json`）的 10-14 至 10-18 部分，
  /// 一行不改。期望值全部来自独立脚本（不复用本模块任何逻辑）。
  const REAL: &[(&str, f64, KpKind)] = &[
    ("2026-10-14T00:00:00", 2.33, O),
    ("2026-10-14T03:00:00", 3.33, O),
    ("2026-10-14T06:00:00", 1.67, O),
    ("2026-10-14T09:00:00", 2.33, O),
    ("2026-10-14T12:00:00", 2.0, O),
    ("2026-10-14T15:00:00", 2.0, O),
    ("2026-10-14T18:00:00", 2.67, O),
    ("2026-10-14T21:00:00", 2.67, O),
    ("2026-10-15T00:00:00", 2.0, O),
    ("2026-10-15T03:00:00", 0.67, O),
    ("2026-10-15T06:00:00", 1.33, O),
    ("2026-10-15T09:00:00", 2.33, E),
    ("2026-10-15T12:00:00", 1.67, E),
    ("2026-10-15T15:00:00", 1.0, E),
    ("2026-10-15T18:00:00", 1.33, E),
    ("2026-10-15T21:00:00", 2.33, E),
    ("2026-10-16T00:00:00", 2.33, P),
    ("2026-10-16T03:00:00", 2.67, P),
    ("2026-10-16T06:00:00", 2.0, P),
    ("2026-10-16T09:00:00", 1.67, P),
    ("2026-10-16T12:00:00", 1.67, P),
    ("2026-10-16T15:00:00", 2.0, P),
    ("2026-10-16T18:00:00", 2.0, P),
    ("2026-10-16T21:00:00", 2.67, P),
    ("2026-10-17T00:00:00", 1.67, P),
    ("2026-10-17T03:00:00", 1.67, P),
    ("2026-10-17T06:00:00", 1.33, P),
    ("2026-10-17T09:00:00", 1.33, P),
    ("2026-10-17T12:00:00", 1.33, P),
    ("2026-10-17T15:00:00", 1.33, P),
    ("2026-10-17T18:00:00", 1.33, P),
    ("2026-10-17T21:00:00", 1.67, P),
    ("2026-10-18T00:00:00", 1.67, P),
  ];

  fn real() -> Vec<KpPoint> {
    REAL.iter().map(|&(t, k, c)| p(t, k, c)).collect()
  }

  fn row(o: &Outlook, date: &str) -> Vec<Option<(f64, KpKind)>> {
    o.days
      .iter()
      .find(|d| d.date == date)
      .unwrap()
      .slots
      .iter()
      .map(|s| s.map(|s| (s.kp, s.kind)))
      .collect()
  }

  #[test]
  fn g_scale_matches_noaas_own_labels_and_the_scale_page() {
    // 上游数据里实际出现过的三个 G 级（2026-10-10）：5.33 → G1、6.67 → G3、7.0 → G3。
    assert_eq!(g_scale(5.33), Some(1));
    assert_eq!(g_scale(6.67), Some(3));
    assert_eq!(g_scale(7.0), Some(3));
    // 同一份数据里其余 78 行都没有 G 级：最高的非 G 值是 4.33 以下。
    for kp in [0.0, 0.67, 1.0, 2.33, 3.33, 3.67, 4.0, 4.33] {
      assert_eq!(g_scale(kp), None, "{kp}");
    }
    // 整数 Kp 四舍五入分档（thirds 记法下 `n-` / `n` / `n+` 同属 n）。
    for (kp, g) in [
      (4.67, 1),
      (5.0, 1),
      (5.67, 2),
      (6.0, 2),
      (6.33, 2),
      (7.33, 3),
      (7.67, 4),
      (8.0, 4),
      (8.33, 4),
    ] {
      assert_eq!(g_scale(kp), Some(g), "{kp}");
    }
    // NOAA：「Kp = 8, including a 9-」——9-（8.67）是 G4，G5 要整 9。
    assert_eq!(g_scale(8.67), Some(4));
    assert_eq!(g_scale(9.0), Some(5));
  }

  #[test]
  fn g_scale_refuses_nonsense() {
    for kp in [f64::NAN, f64::INFINITY, -0.1, 9.01, 12.0] {
      assert_eq!(g_scale(kp), None, "{kp}");
    }
  }

  #[test]
  fn levels_split_quiet_active_and_storm() {
    assert_eq!(level(3.33), KpLevel::Quiet); // 3+
    assert_eq!(level(3.67), KpLevel::Active); // 4-：整数 Kp 取四舍五入就是 4
    assert_eq!(level(4.0), KpLevel::Active);
    assert_eq!(level(4.33), KpLevel::Active);
    assert_eq!(level(4.67), KpLevel::Storm(1)); // 5-
    assert_eq!(level(7.0), KpLevel::Storm(3));
    assert_eq!(level(f64::NAN), KpLevel::Quiet);
  }

  #[test]
  fn the_real_feed_lays_out_as_four_days_like_noaas_text_plus_yesterday() {
    let o = outlook(&real()).unwrap();
    // 「今天」是第一个非实测时段所在的天（10-15），展示昨天到后天；10-18 那条孤零零的 00:00 不展示。
    let dates: Vec<_> = o.days.iter().map(|d| d.date.as_str()).collect();
    assert_eq!(
      dates,
      ["2026-10-14", "2026-10-15", "2026-10-16", "2026-10-17"]
    );

    let kp = |d: &str| -> Vec<f64> { row(&o, d).into_iter().map(|s| s.unwrap().0).collect() };
    assert_eq!(
      kp("2026-10-14"),
      [2.33, 3.33, 1.67, 2.33, 2.0, 2.0, 2.67, 2.67]
    );
    assert_eq!(
      kp("2026-10-15"),
      [2.0, 0.67, 1.33, 2.33, 1.67, 1.0, 1.33, 2.33]
    );
    assert_eq!(
      kp("2026-10-16"),
      [2.33, 2.67, 2.0, 1.67, 1.67, 2.0, 2.0, 2.67]
    );
    assert_eq!(
      kp("2026-10-17"),
      [1.67, 1.67, 1.33, 1.33, 1.33, 1.33, 1.33, 1.67]
    );

    // 同一天里实测 / 估计 / 预报各占各的格。
    let kinds: Vec<_> = row(&o, "2026-10-15").iter().map(|s| s.unwrap().1).collect();
    assert_eq!(kinds, [O, O, O, E, E, E, E, E]);
    assert!(row(&o, "2026-10-16").iter().all(|s| s.unwrap().1 == P));
  }

  #[test]
  fn the_peak_is_the_highest_non_observed_slot_earliest_on_a_tie() {
    let o = outlook(&real()).unwrap();
    let peak = o.peak.unwrap();
    // 10-16 的 03–06 与 21–00 都是 2.67，取最早的；10-14 的实测 3.33 更高，但那是历史，不算预报。
    assert_eq!(
      (peak.kp, peak.date.as_str(), peak.slot),
      (2.67, "2026-10-16", 1)
    );
    assert_eq!(SLOT_LABELS[peak.slot], "03–06");
    assert_eq!(peak.g, None);
  }

  #[test]
  fn a_storm_in_the_forecast_carries_its_g_level() {
    let mut pts = real();
    for x in &mut pts {
      if x.time == "2026-10-17T06:00:00" {
        x.kp = 6.67;
      }
    }
    let peak = outlook(&pts).unwrap().peak.unwrap();
    assert_eq!(
      (peak.kp, peak.date.as_str(), peak.slot, peak.g),
      (6.67, "2026-10-17", 2, Some(3))
    );
    // 实测里出现磁暴不会被当成预报。
    let mut pts = real();
    pts[1].kp = 7.0;
    assert_eq!(outlook(&pts).unwrap().peak.unwrap().g, None);
  }

  #[test]
  fn when_everything_is_observed_there_is_no_peak_and_the_last_day_anchors_the_window() {
    let pts: Vec<_> = REAL
      .iter()
      .filter(|r| r.2 == O)
      .map(|&(t, k, c)| p(t, k, c))
      .collect();
    let o = outlook(&pts).unwrap();
    assert!(o.peak.is_none());
    let dates: Vec<_> = o.days.iter().map(|d| d.date.as_str()).collect();
    // D0 取最后一天（10-15）：昨天与今天有数据，后两天没有、不出现。
    assert_eq!(dates, ["2026-10-14", "2026-10-15"]);
  }

  #[test]
  fn empty_and_all_invalid_input_gives_nothing() {
    assert_eq!(outlook(&[]), None);
    assert_eq!(
      outlook(&[p("garbage", 1.0, O), p("2026-10-15T00:00:00", f64::NAN, P)]),
      None
    );
  }

  #[test]
  fn clean_drops_bad_rows_normalises_times_and_lets_later_rows_win() {
    let (out, dropped) = clean(vec![
      p("2026-10-15T03:00:00", 2.0, P),
      p("2026-10-15T00:00", 1.0, O),       // 没写秒：规范成 :00:00
      p("2026-10-15T03:00", 3.0, E),       // 与第一条同一时段，后面的覆盖前面的
      p("2026-02-30T00:00:00", 1.0, O),    // 日历上不存在
      p("2026-10-15T04:00:00", 1.0, O),    // 不是 3 的倍数
      p("2026-10-15T03:30:00", 1.0, O),    // 分不是 0
      p("2026-10-15T06:00:15", 1.0, O),    // 秒不是 0
      p("2026-10-15T24:00:00", 1.0, O),    // 时越界
      p("2026-10-15T09:00:00:00", 1.0, O), // 多一段
      p("2026-10-15T12:00:00", 9.5, O),    // Kp 越界
      p("2026-10-15T15:00:00", -1.0, O),
      p("2026-10-15T18:00:00", f64::INFINITY, O),
      p("not-a-time", 1.0, O),
      p("", 1.0, O),
      p("2026-10-15", 1.0, O),          // 没有时间部分
      p("2026-10-15T21:00:00", 9.0, P), // 边界值合法
      p("2026-10-15T06:00:00", 0.0, O), // 边界值合法
    ]);
    let got: Vec<_> = out
      .iter()
      .map(|x| (x.time.as_str(), x.kp, x.kind))
      .collect();
    assert_eq!(
      got,
      [
        ("2026-10-15T00:00:00", 1.0, O),
        ("2026-10-15T03:00:00", 3.0, E),
        ("2026-10-15T06:00:00", 0.0, O),
        ("2026-10-15T21:00:00", 9.0, P),
      ]
    );
    // 17 行进，4 行出，丢了 13 行（含被覆盖的 1 行）。
    assert_eq!(dropped, 13);
  }

  #[test]
  fn clean_sorts_chronologically_even_across_month_and_year_ends() {
    let (out, _) = clean(vec![
      p("2027-01-01T00:00:00", 1.0, P),
      p("2026-12-31T21:00:00", 2.0, E),
      p("2026-12-31T00:00:00", 3.0, O),
    ]);
    let times: Vec<_> = out.iter().map(|x| x.time.as_str()).collect();
    assert_eq!(
      times,
      [
        "2026-12-31T00:00:00",
        "2026-12-31T21:00:00",
        "2027-01-01T00:00:00"
      ]
    );
  }

  #[test]
  fn the_window_crosses_month_and_leap_day_boundaries() {
    let pts = vec![
      p("2026-10-31T21:00:00", 2.0, O),
      p("2026-11-01T00:00:00", 3.0, P),
      p("2026-11-02T06:00:00", 4.0, P),
      p("2026-11-03T12:00:00", 5.0, P),
      p("2026-11-04T00:00:00", 9.0, P), // D0+3：窗口之外，不进峰值也不展示
    ];
    let o = outlook(&pts).unwrap();
    let dates: Vec<_> = o.days.iter().map(|d| d.date.as_str()).collect();
    assert_eq!(
      dates,
      ["2026-10-31", "2026-11-01", "2026-11-02", "2026-11-03"]
    );
    assert_eq!(o.peak.unwrap().kp, 5.0);

    // 闰日：2028-02-29 存在，2027-02-29 不存在。
    let leap = vec![
      p("2028-02-28T21:00:00", 1.0, O),
      p("2028-02-29T00:00:00", 1.0, P),
      p("2028-03-01T00:00:00", 1.0, P),
    ];
    let dates: Vec<_> = outlook(&leap)
      .unwrap()
      .days
      .into_iter()
      .map(|d| d.date)
      .collect();
    assert_eq!(dates, ["2028-02-28", "2028-02-29", "2028-03-01"]);
    assert!(split_time("2027-02-29T00:00:00").is_none());
    assert!(split_time("2028-02-29T00:00:00").is_some());
  }

  #[test]
  fn a_missing_middle_day_just_disappears_instead_of_showing_an_empty_column() {
    let pts = vec![
      p("2026-10-15T00:00:00", 1.0, E),
      p("2026-10-17T00:00:00", 2.0, P),
    ];
    let o = outlook(&pts).unwrap();
    let dates: Vec<_> = o.days.iter().map(|d| d.date.as_str()).collect();
    assert_eq!(dates, ["2026-10-15", "2026-10-17"]);
  }

  #[test]
  fn input_order_does_not_matter() {
    let mut shuffled = real();
    shuffled.reverse();
    assert_eq!(outlook(&shuffled), outlook(&real()));
  }

  #[test]
  fn upstream_kinds_and_json_round_trip() {
    assert_eq!(KpKind::from_upstream("observed"), Some(O));
    assert_eq!(KpKind::from_upstream("estimated"), Some(E));
    assert_eq!(KpKind::from_upstream("predicted"), Some(P));
    for bad in ["", "Observed", "forecast", "predicted "] {
      assert_eq!(KpKind::from_upstream(bad), None, "{bad:?}");
    }
    let f = KpForecast {
      source: "NOAA SWPC".into(),
      updated: Some("2026-10-15T06:00:00".into()),
      points: vec![p("2026-10-15T09:00:00", 2.33, E)],
    };
    let json = serde_json::to_string(&f).unwrap();
    assert!(json.contains(r#""kind":"estimated""#), "{json}");
    assert_eq!(serde_json::from_str::<KpForecast>(&json).unwrap(), f);
  }

  #[test]
  fn slot_labels_cover_the_day() {
    assert_eq!(SLOT_LABELS.len(), SLOTS_PER_DAY);
    assert_eq!(SLOT_LABELS[0], "00–03");
    assert_eq!(SLOT_LABELS[7], "21–00");
  }
}
