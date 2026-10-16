//! 竞赛赛后分析：每小时速率、乘数缺口、休息（空闲）时段。
//!
//! 只读分析，不改日志。「哪些算有效通联」「每条通联带来哪些乘数」直接复用
//! [`crate::contest`] 的记分规则，所以这里的数字与自报分数口径一致。
//!
//! 时间一律是 UTC、精度为分钟（日志本身只存到分钟）。日期或时间解析不出的通联不进时间类统计
//! （速率、空闲），但仍参与乘数统计，数量记在 [`Analysis::untimed`]。

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::bands::AMATEUR_BAND_EDGES;
use crate::contest::{ContestDef, Mult, MultKind, Scoring, qso_value, valid_qsos};
use crate::dxcc::lookup;
use crate::logbook::LogEntry;

/// 每小时表最多补全的小时数：跨度更长（日志里混进了错误日期）时只列有通联的小时，
/// 不为中间几十万个空小时分配内存。
const MAX_FILLED_HOURS: i64 = 240;

/// CQ 分区总数（分区缺口的全集 1..=40）。
const ZONES: u8 = 40;

/// 一个 UTC 整点小时。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HourRow {
  /// 该小时起点（自 1970-01-01 起的分钟数，可用 [`crate::logbook::stamp`] 还原）。
  pub start: i64,
  pub qsos: u32,
  pub points: u32,
  /// 这个小时里**首次**拿到的乘数个数。
  pub new_mults: u32,
}

/// 任意连续 60 分钟里通联最多的窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestWindow {
  /// 窗口起点，恰好是某条通联的时刻；窗口为 `[start, start + 60)`。
  pub start: i64,
  pub qsos: u32,
}

/// 相邻两条通联之间的空闲时段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleGap {
  /// 前一条通联的时刻。
  pub from: i64,
  /// 后一条通联的时刻。
  pub to: i64,
}

impl IdleGap {
  #[must_use]
  pub const fn minutes(&self) -> i64 {
    self.to - self.from
  }
}

/// 一个按波段计的乘数：在部分已用波段拿到了，另一些已用波段还没有。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossBandGap {
  /// `band` 恒为 `None`：这条乘数跨波段汇总。
  pub mult: Mult,
  pub worked: Vec<String>,
  pub missing: Vec<String>,
}

/// 某个波段上还没拿到的 CQ 分区。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandZones {
  pub band: String,
  pub worked: u32,
  /// 升序。
  pub missing: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
  /// 有效（非重复）通联数。
  pub valid: u32,
  /// 日期或时间解析不出、没进时间类统计的通联数。
  pub untimed: u32,
  /// 用过的波段，按频率从低到高。
  pub bands: Vec<String>,
  /// 这个竞赛是否有「按波段计」的乘数（分区 / 实体 / 州）。没有时（如 WPX 的前缀）就谈不上跨波段缺口。
  pub per_band_mults: bool,
  /// 第一条与最后一条有时间的通联时刻；一条都没有则为 `None`。
  pub span: Option<(i64, i64)>,
  /// 逐小时（升序）。跨度不超过 [`MAX_FILLED_HOURS`] 时补全中间的空小时，否则只列有通联的小时。
  pub hours: Vec<HourRow>,
  pub best_hour: Option<BestWindow>,
  /// 不短于阈值的空闲段（升序）。
  pub idle: Vec<IdleGap>,
  /// 空闲段总分钟数。
  pub idle_minutes: i64,
  /// 跨波段乘数缺口：缺的波段越少越靠前（最接近补齐的先列），其次按种类、值排序。
  pub cross_band: Vec<CrossBandGap>,
  /// 各已用波段上缺的 CQ 分区；只有以 CQ 分区为乘数的竞赛才有。
  pub zones: Vec<BandZones>,
}

fn band_rank(band: &str) -> usize {
  AMATEUR_BAND_EDGES
    .iter()
    .position(|(name, _, _)| *name == band)
    .unwrap_or(usize::MAX)
}

fn sort_bands(bands: &mut [String]) {
  bands.sort_by(|a, b| band_rank(a).cmp(&band_rank(b)).then_with(|| a.cmp(b)));
}

/// 以 CQ 分区为乘数的竞赛（分区全集已知，才有「缺哪些分区」）。
const fn uses_zones(s: Scoring) -> bool {
  matches!(s, Scoring::CqWw | Scoring::ZoneMults)
}

/// 乘数按波段分别计的竞赛。
const fn has_band_mults(s: Scoring) -> bool {
  matches!(
    s,
    Scoring::CqWw | Scoring::ArrlDx | Scoring::DxccPerBand | Scoring::ZoneMults
  )
}

/// 分析一场竞赛。没有任何有效通联时返回 `None`。
///
/// `min_idle` 是空闲段的最短分钟数（小于 1 按 1 处理）。`my_call` 用于判断得分（与 [`crate::contest::score`] 相同）。
#[must_use]
pub fn analyze(
  def: &ContestDef,
  entries: &[LogEntry],
  my_call: &str,
  min_idle: i64,
) -> Option<Analysis> {
  let me = lookup(my_call);
  let (mut valid, _dupes) = valid_qsos(def, entries);
  if valid.is_empty() {
    return None;
  }
  // 首次乘数按真实时刻归属；无法定时的记录放最后，不能抢走有时间记录的首次贡献。
  valid.sort_by_key(|e| e.minutes_on().unwrap_or(i64::MAX));

  let mut seen: HashSet<Mult> = HashSet::new();
  // 小时起点 -> (通联, 得分, 新乘数)
  let mut by_hour: BTreeMap<i64, (u32, u32, u32)> = BTreeMap::new();
  let mut times: Vec<i64> = Vec::new();
  let mut untimed = 0u32;
  // 按波段计的乘数 -> 已拿到的波段
  let mut worked: BTreeMap<Mult, BTreeSet<String>> = BTreeMap::new();
  let mut used_bands: BTreeSet<String> = BTreeSet::new();

  for e in &valid {
    let (points, mults) = qso_value(def, me, e);
    let band = e.band_label();
    if !band.is_empty() {
      used_bands.insert(band);
    }
    let new = mults.iter().filter(|m| seen.insert((*m).clone())).count() as u32;
    for m in mults {
      if let Some(b) = &m.band {
        let key = Mult {
          kind: m.kind,
          band: None,
          value: m.value.clone(),
        };
        worked.entry(key).or_default().insert(b.clone());
      }
    }
    match e.minutes_on() {
      Some(t) => {
        times.push(t);
        let row = by_hour.entry(t.div_euclid(60) * 60).or_default();
        row.0 += 1;
        row.1 += points;
        row.2 += new;
      }
      None => untimed += 1,
    }
  }

  // `valid` 是按（日期文本，时间文本，id）排的；日期时间写法一致时就是时间序，
  // 但 `8:30` 这类写法会让文本序与时间序不一致，所以这里再按真实时刻排一次。
  times.sort_unstable();

  let span = times.first().zip(times.last()).map(|(a, b)| (*a, *b));
  let hours = build_hours(&by_hour);
  let best_hour = best_window(&times);
  let min_idle = min_idle.max(1);
  let idle: Vec<IdleGap> = times
    .windows(2)
    .filter(|w| w[1] - w[0] >= min_idle)
    .map(|w| IdleGap {
      from: w[0],
      to: w[1],
    })
    .collect();
  let idle_minutes = idle.iter().map(IdleGap::minutes).sum();

  let mut cross_band = Vec::new();
  for (mult, got) in worked {
    let missing: Vec<String> = used_bands
      .iter()
      .filter(|b| !got.contains(*b))
      .cloned()
      .collect();
    if got.is_empty() || missing.is_empty() {
      continue;
    }
    let mut worked_bands: Vec<String> = got.into_iter().collect();
    let mut missing = missing;
    sort_bands(&mut worked_bands);
    sort_bands(&mut missing);
    cross_band.push(CrossBandGap {
      mult,
      worked: worked_bands,
      missing,
    });
  }
  cross_band.sort_by(|a, b| {
    a.missing
      .len()
      .cmp(&b.missing.len())
      .then_with(|| a.mult.kind.cmp(&b.mult.kind))
      .then_with(|| cmp_value(&a.mult, &b.mult))
  });

  let zones = if uses_zones(def.scoring) {
    zone_gaps(&seen, &used_bands)
  } else {
    Vec::new()
  };

  let mut bands: Vec<String> = used_bands.iter().cloned().collect();
  sort_bands(&mut bands);

  Some(Analysis {
    valid: valid.len() as u32,
    untimed,
    bands,
    per_band_mults: has_band_mults(def.scoring),
    span,
    hours,
    best_hour,
    idle,
    idle_minutes,
    cross_band,
    zones,
  })
}

/// 值的排序：分区按数值（`9` 排在 `14` 前），其余按文本。
fn cmp_value(a: &Mult, b: &Mult) -> std::cmp::Ordering {
  match (a.value.parse::<u32>(), b.value.parse::<u32>()) {
    (Ok(x), Ok(y)) => x.cmp(&y),
    _ => a.value.cmp(&b.value),
  }
}

fn build_hours(by_hour: &BTreeMap<i64, (u32, u32, u32)>) -> Vec<HourRow> {
  let (Some((&first, _)), Some((&last, _))) = (by_hour.first_key_value(), by_hour.last_key_value())
  else {
    return Vec::new();
  };
  let row = |start: i64| {
    let (qsos, points, new_mults) = by_hour.get(&start).copied().unwrap_or_default();
    HourRow {
      start,
      qsos,
      points,
      new_mults,
    }
  };
  if (last - first) / 60 < MAX_FILLED_HOURS {
    (first..=last).step_by(60).map(row).collect()
  } else {
    by_hour.keys().map(|&h| row(h)).collect()
  }
}

/// 双指针找任意 60 分钟窗口 `[t, t+60)` 内通联最多的那个；并列取最早。`times` 须升序。
fn best_window(times: &[i64]) -> Option<BestWindow> {
  let mut best: Option<BestWindow> = None;
  let mut hi = 0;
  for (lo, &start) in times.iter().enumerate() {
    while hi < times.len() && times[hi] < start + 60 {
      hi += 1;
    }
    let qsos = (hi - lo) as u32;
    if best.is_none_or(|b| qsos > b.qsos) {
      best = Some(BestWindow { start, qsos });
    }
  }
  best
}

fn zone_gaps(seen: &HashSet<Mult>, used_bands: &BTreeSet<String>) -> Vec<BandZones> {
  let mut out: Vec<BandZones> = used_bands
    .iter()
    .map(|band| {
      let got: BTreeSet<u8> = seen
        .iter()
        .filter(|m| m.kind == MultKind::Zone && m.band.as_deref() == Some(band))
        .filter_map(|m| m.value.parse().ok())
        .collect();
      BandZones {
        band: band.clone(),
        worked: got.len() as u32,
        missing: (1..=ZONES).filter(|z| !got.contains(z)).collect(),
      }
    })
    .collect();
  out.sort_by_key(|z| band_rank(&z.band));
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contest::contest;
  use crate::logbook::minutes_of;

  #[test]
  fn first_multipliers_follow_real_time() {
    let entries = [
      q(1, "8:30", "W1AW", "14.021", "5"),
      q(2, "10:00", "W2AW", "14.021", "5"),
    ];
    let result = analyze(contest("CQ-WW-CW").unwrap(), &entries, "BG4ABC", 30).unwrap();
    assert_eq!(result.hours[0].new_mults, 2);
    assert_eq!(result.hours[2].new_mults, 0);
  }

  fn q(id: u64, time: &str, call: &str, freq: &str, zone: &str) -> LogEntry {
    LogEntry {
      id,
      date: "2026-10-31".into(),
      time: time.into(),
      freq: freq.into(),
      mode: "CW".into(),
      callsign: call.into(),
      srx: zone.into(),
      ..Default::default()
    }
  }

  /// 数据与期望值都来自独立脚本（不复用本模块任何逻辑）：
  /// 9 条里第 5 条是 20m 上 JA1ABC 的重复；本台是中国（亚洲）。
  fn log() -> Vec<LogEntry> {
    vec![
      q(1, "00:10", "W1AW", "14.020", "5"),
      q(2, "00:20", "JA1ABC", "14.021", "25"),
      q(3, "00:50", "DL1ABC", "14.022", "14"),
      q(4, "01:05", "W1AW", "7.010", "5"),
      q(5, "01:15", "JA1ABC", "14.021", "25"),
      q(6, "01:30", "JA1ABC", "7.011", "25"),
      q(7, "02:40", "DL1ABC", "7.012", "14"),
      q(8, "02:45", "W1AW", "21.020", "5"),
      q(9, "02:50", "JA1ABC", "21.021", "25"),
    ]
  }

  fn day() -> i64 {
    minutes_of("2026-10-31", "00:00").unwrap()
  }

  fn cqww() -> &'static ContestDef {
    contest("CQ-WW-CW").unwrap()
  }

  #[test]
  fn hourly_rows_match_the_independent_calculation() {
    let a = analyze(cqww(), &log(), "BG4ABC", 30).unwrap();
    assert_eq!((a.valid, a.untimed), (8, 0));
    assert_eq!(a.span, Some((day() + 10, day() + 170)));
    // 波段按频率从低到高，不是按字母序（字母序会把 15m 排在最前）。
    assert_eq!(a.bands, ["40m", "20m", "15m"]);
    assert!(a.per_band_mults);
    // 每小时：(通联, 得分, 首次拿到的乘数)；总乘数 16，总得分 18。
    let rows: Vec<_> = a
      .hours
      .iter()
      .map(|h| ((h.start - day()) / 60, h.qsos, h.points, h.new_mults))
      .collect();
    assert_eq!(rows, vec![(0, 3, 7, 6), (1, 2, 4, 4), (2, 3, 7, 6)]);
    assert_eq!(a.hours.iter().map(|h| h.new_mults).sum::<u32>(), 16);
    assert_eq!(a.hours.iter().map(|h| h.points).sum::<u32>(), 18);
  }

  #[test]
  fn per_hour_totals_agree_with_the_scoring_function() {
    // 同一份日志：赛后分析的合计必须等于记分函数的结果，两边不能各算各的。
    let s = crate::contest::score(cqww(), &log(), "BG4ABC");
    let a = analyze(cqww(), &log(), "BG4ABC", 30).unwrap();
    assert_eq!(a.hours.iter().map(|h| h.qsos).sum::<u32>(), s.qsos);
    assert_eq!(a.hours.iter().map(|h| h.points).sum::<u32>(), s.points);
    assert_eq!(a.hours.iter().map(|h| h.new_mults).sum::<u32>(), s.mults);
    assert_eq!(a.valid, s.qsos);
  }

  #[test]
  fn best_sixty_minute_window_is_any_window_not_a_clock_hour() {
    let a = analyze(cqww(), &log(), "BG4ABC", 30).unwrap();
    // 00:10 起的 60 分钟含 00:10 / 00:20 / 00:50 / 01:05 共 4 条；整点小时最多只有 3 条。
    assert_eq!(
      a.best_hour,
      Some(BestWindow {
        start: day() + 10,
        qsos: 4
      })
    );
    assert!(a.hours.iter().all(|h| h.qsos <= 3));
  }

  #[test]
  fn best_window_edges() {
    assert_eq!(best_window(&[]), None);
    assert_eq!(best_window(&[7]), Some(BestWindow { start: 7, qsos: 1 }));
    // 窗口是左闭右开：相隔恰好 60 分钟的两条不在同一窗口。
    assert_eq!(best_window(&[0, 60]).unwrap().qsos, 1);
    assert_eq!(best_window(&[0, 59]).unwrap().qsos, 2);
    // 并列取最早。
    assert_eq!(best_window(&[0, 200]).unwrap().start, 0);
  }

  #[test]
  fn idle_gaps_follow_the_threshold() {
    let at = |th| analyze(cqww(), &log(), "BG4ABC", th).unwrap();
    let pairs = |a: &Analysis| -> Vec<(i64, i64)> {
      a.idle
        .iter()
        .map(|g| (g.from - day(), g.to - day()))
        .collect()
    };
    let a = at(30);
    assert_eq!(pairs(&a), vec![(20, 50), (90, 160)]); // 恰好 30 分钟也算
    assert_eq!(a.idle_minutes, 100);
    let a = at(60);
    assert_eq!(pairs(&a), vec![(90, 160)]);
    assert_eq!(a.idle_minutes, 70);
    let a = at(15);
    assert_eq!(pairs(&a), vec![(20, 50), (50, 65), (65, 90), (90, 160)]);
    assert_eq!(a.idle_minutes, 140);
    // 阈值不合法（0 / 负数）按 1 分钟处理，不 panic 也不把「零间隔」算成空闲。
    assert_eq!(at(0).idle, at(1).idle);
    assert_eq!(at(-5).idle, at(1).idle);
  }

  #[test]
  fn cross_band_gaps_list_only_mults_worked_on_some_but_not_all_used_bands() {
    let a = analyze(cqww(), &log(), "BG4ABC", 30).unwrap();
    let got: Vec<_> = a
      .cross_band
      .iter()
      .map(|g| {
        (
          g.mult.kind,
          g.mult.value.as_str(),
          g.worked.clone(),
          g.missing.clone(),
        )
      })
      .collect();
    let de = crate::dxcc::lookup("DL1ABC").unwrap().dxcc.to_string();
    // 分区 14 与德国在 20m / 40m 拿到、15m 没有；其余乘数三个波段都有，不应出现。
    assert_eq!(
      got,
      vec![
        (
          MultKind::Zone,
          "14",
          vec!["40m".to_owned(), "20m".to_owned()],
          vec!["15m".to_owned()]
        ),
        (
          MultKind::Entity,
          de.as_str(),
          vec!["40m".to_owned(), "20m".to_owned()],
          vec!["15m".to_owned()]
        ),
      ]
    );
    assert!(a.cross_band.iter().all(|g| g.mult.band.is_none()));
  }

  #[test]
  fn missing_zones_per_used_band() {
    let a = analyze(cqww(), &log(), "BG4ABC", 30).unwrap();
    let by: Vec<_> = a
      .zones
      .iter()
      .map(|z| (z.band.as_str(), z.worked, z.missing.len()))
      .collect();
    // 波段按频率从低到高；40m 在 20m 前、15m 在最后。
    assert_eq!(by, vec![("40m", 3, 37), ("20m", 3, 37), ("15m", 2, 38)]);
    let m15 = &a.zones[2].missing;
    assert!(m15.contains(&1) && m15.contains(&14) && !m15.contains(&5) && !m15.contains(&25));
    assert!(m15.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(m15.len() + 2, 40);
  }

  #[test]
  fn contests_without_a_known_universe_get_no_zone_gaps() {
    let wpx = contest("CQ-WPX-CW").unwrap();
    let a = analyze(wpx, &log(), "BG4ABC", 30).unwrap();
    // WPX 的乘数是前缀，不分波段、也没有已知全集：既没有分区缺口，也没有跨波段缺口。
    assert!(a.zones.is_empty());
    assert!(a.cross_band.is_empty());
    assert!(!a.per_band_mults);
    // 但速率与空闲照常算。
    assert_eq!(a.hours.len(), 3);
    assert_eq!(a.idle.len(), 2);

    // JIDX（分区乘数）有分区缺口；ARRL DX 的州乘数只有跨波段缺口。
    let jidx = contest("JIDX-CW").unwrap();
    assert!(
      !analyze(jidx, &log(), "BG4ABC", 30)
        .unwrap()
        .zones
        .is_empty()
    );
    let arrl = contest("ARRL-DX-CW").unwrap();
    let a = analyze(arrl, &log(), "BG4ABC", 30).unwrap();
    assert!(a.zones.is_empty());
  }

  #[test]
  fn empty_and_all_dupe_logs() {
    assert_eq!(analyze(cqww(), &[], "BG4ABC", 30), None);
    let one = vec![q(1, "00:10", "W1AW", "14.020", "5")];
    let a = analyze(cqww(), &one, "BG4ABC", 30).unwrap();
    assert_eq!(
      (a.valid, a.hours.len(), a.idle.len(), a.idle_minutes),
      (1, 1, 0, 0)
    );
    assert_eq!(a.best_hour.map(|b| b.qsos), Some(1));
    assert!(a.cross_band.is_empty());
  }

  #[test]
  fn untimed_qsos_count_for_mults_but_not_for_time_statistics() {
    let mut l = log();
    // 缺时间：仍是有效通联、仍带来乘数，但不进速率与空闲。
    l.push(q(10, "", "VK2XX", "14.030", "30"));
    let a = analyze(cqww(), &l, "BG4ABC", 30).unwrap();
    assert_eq!((a.valid, a.untimed), (9, 1));
    assert_eq!(a.hours.iter().map(|h| h.qsos).sum::<u32>(), 8);
    let s = crate::contest::score(cqww(), &l, "BG4ABC");
    assert_eq!(s.qsos, 9);
    // 20m 分区：5 / 25 / 14 / 30 → 4 个。
    let z20 = a.zones.iter().find(|z| z.band == "20m").unwrap();
    assert_eq!(z20.worked, 4);
    assert!(!z20.missing.contains(&30));
  }

  #[test]
  fn hours_are_filled_across_empty_stretches_until_the_span_gets_silly() {
    let l = vec![
      q(1, "00:10", "W1AW", "14.020", "5"),
      q(2, "05:30", "JA1ABC", "14.021", "25"),
    ];
    let a = analyze(cqww(), &l, "BG4ABC", 30).unwrap();
    // 00 到 05 点共 6 个小时，中间 4 个空小时也有行。
    assert_eq!(a.hours.len(), 6);
    assert_eq!(
      a.hours.iter().map(|h| h.qsos).collect::<Vec<_>>(),
      [1, 0, 0, 0, 0, 1]
    );
    assert_eq!(a.idle_minutes, 320);

    // 日志里混进一条错误日期：跨度远超上限，只列有通联的小时，不分配几十万行。
    let mut far = l.clone();
    far.push(LogEntry {
      date: "2099-01-01".into(),
      ..q(3, "00:00", "DL1ABC", "14.022", "14")
    });
    let a = analyze(cqww(), &far, "BG4ABC", 30).unwrap();
    assert_eq!(a.hours.len(), 3);
    assert!(a.hours.iter().all(|h| h.qsos == 1));
  }

  #[test]
  fn unpadded_times_sort_by_real_time_not_by_text() {
    // 文本序里 "8:30" 排在 "10:00" 之后，真实时间序相反。
    let l = vec![
      q(1, "8:30", "W1AW", "14.020", "5"),
      q(2, "10:00", "JA1ABC", "14.021", "25"),
    ];
    let a = analyze(cqww(), &l, "BG4ABC", 30).unwrap();
    assert_eq!(a.idle.len(), 1);
    assert_eq!(a.idle[0].minutes(), 90);
    assert_eq!(a.span, Some((day() + 8 * 60 + 30, day() + 10 * 60)));
  }
}
