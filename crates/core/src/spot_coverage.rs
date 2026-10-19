//! 接收报告的方位聚合：把「谁收到了我」按 8 个方位扇区与波段分账。
//!
//! 这是「我的天线与传播到底往哪儿打得出去」的实测答案：数据来自 RBN / PSK Reporter
//! 的接收台网格，不涉及任何预测模型。扇区是**以本台为原点**的方位，不是目标的
//! 地理方位 —— 对本台八木的朝向判断才有意义。

use crate::grid::{distance_bearing, lat_lon_from_grid};

/// 方位扇区数（每 45° 一个，0 号扇区以正北为中心）。
pub const SECTORS: usize = 8;

/// 一条可聚合的接收报告。
#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
  /// 接收台网格（Maidenhead）；空或非法时该条不参与方位统计。
  pub locator: String,
  /// 波段名（`20m`、`40m`…），只是分组标签，不解析。
  pub band: String,
  /// 信噪比（dB），可正可负。
  pub snr: f64,
  /// 报告时刻的 UTC 小时（0–23）；越界的值按取模处理。
  ///
  /// `None` = 数据源没有时间戳（如 RBN）。未知时刻**不计入** `hourly`：硬填 0 会让
  /// 全部报告堆在 0 点，`busiest_hour()` 因此给出错误答案。
  pub hour_utc: Option<u8>,
}

/// 一个扇区的统计。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectorStat {
  pub count: usize,
  /// 平均 SNR；`count == 0` 时为 0（页面应显示「—」）。
  ///
  /// 这是 **dB 的算术平均**（等价于线性功率的几何平均），不是平均功率：`-10` 与 `-20`
  /// 得 `-15`，而线性功率平均约 `-12.6 dB`。只作相对强弱的参考。
  pub mean_snr: f64,
}

impl SectorStat {
  const EMPTY: Self = Self {
    count: 0,
    mean_snr: 0.0,
  };
}

/// 聚合结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Coverage {
  /// 参与统计的报告数（网格无效的已剔除）。
  pub total: usize,
  pub sectors: [SectorStat; SECTORS],
  /// 按 UTC 小时统计的报告数（索引 0–23）。时段是传播窗口最直接的证据：
  /// 同一个方向，早上和傍晚被听到的概率可以差一个量级。
  pub hourly: [usize; 24],
  /// 按波段分组，按报告数从多到少排列。
  pub by_band: Vec<(String, [SectorStat; SECTORS])>,
}

impl Coverage {
  /// 报告最多的小时；一条都没有时返回 `None`。
  #[must_use]
  pub fn busiest_hour(&self) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for (hour, &count) in self.hourly.iter().enumerate() {
      if count == 0 {
        continue;
      }
      if best.is_none_or(|(_, current)| count > current) {
        best = Some((hour, count));
      }
    }
    best.map(|(hour, _)| hour)
  }
}

/// 方位角 → 扇区号（0 = 北，1 = 东北，顺时针）。
///
/// 非有限值（NaN / inf）会饱和成 0（正北），调用方应先过滤 —— [`aggregate`] 已经这么做。
/// 扇区宽度由 [`SECTORS`] 决定，改常量时不必同步改这里。
#[must_use]
pub fn sector_of_bearing(bearing_deg: f64) -> usize {
  (((bearing_deg.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize) % SECTORS
}

/// 扇区中心方位角（0 = 北，顺时针）。
#[must_use]
pub fn sector_center_deg(sector: usize) -> f64 {
  (sector % SECTORS) as f64 * 45.0
}

fn accumulate(stats: &mut [SectorStat; SECTORS], sector: usize, snr: f64) {
  let stat = &mut stats[sector];
  // 均值用增量更新，避免再存一份总和数组。
  stat.mean_snr = (stat.mean_snr * stat.count as f64 + snr) / (stat.count + 1) as f64;
  stat.count += 1;
}

/// 按本台网格聚合全部报告。
///
/// 本台网格非法时返回 `None`（没有原点就算不出方位）；单条报告的网格非法时
/// 只跳过那一条，不连累整体。
#[must_use]
pub fn aggregate(home_locator: &str, spots: &[Spot]) -> Option<Coverage> {
  let (home_lat, home_lon) = lat_lon_from_grid(home_locator)?;
  let mut sectors = [SectorStat::EMPTY; SECTORS];
  let mut hourly = [0usize; 24];
  let mut by_band: Vec<(String, [SectorStat; SECTORS])> = Vec::new();
  let mut total = 0;

  for spot in spots {
    let Some((lat, lon)) = lat_lon_from_grid(&spot.locator) else {
      continue;
    };
    // 增量均值对 NaN / inf 不可恢复：一条脏 SNR 会让该扇区、该波段的均值永久为 NaN。
    // 整条跳过（含 `total` 与 `hourly`），口径才一致。
    if !spot.snr.is_finite() {
      continue;
    }
    let (_, bearing) = distance_bearing(home_lat, home_lon, lat, lon);
    // 非有限方位会在 `sector_of_bearing` 里静默饱和成 0（正北），脏数据不能就这么算进去。
    if !bearing.is_finite() {
      continue;
    }
    let sector = sector_of_bearing(bearing);
    accumulate(&mut sectors, sector, spot.snr);
    if let Some(hour) = spot.hour_utc {
      hourly[usize::from(hour % 24)] += 1;
    }
    total += 1;

    match by_band.iter_mut().find(|(band, _)| *band == spot.band) {
      Some((_, table)) => accumulate(table, sector, spot.snr),
      None => {
        let mut table = [SectorStat::EMPTY; SECTORS];
        accumulate(&mut table, sector, spot.snr);
        by_band.push((spot.band.clone(), table));
      }
    }
  }

  // `sort_by_cached_key` 每个元素只算一次键；`sort_by` 的比较器会被调用 O(n log n) 次，
  // 每次都要重扫 8 个扇区。
  by_band.sort_by_cached_key(|(_, table)| {
    std::cmp::Reverse(table.iter().map(|x| x.count).sum::<usize>())
  });

  Some(Coverage {
    total,
    sectors,
    hourly,
    by_band,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn spot(locator: &str, band: &str, snr: f64, hour_utc: u8) -> Spot {
    Spot {
      locator: locator.into(),
      band: band.into(),
      snr,
      hour_utc: Some(hour_utc),
    }
  }

  #[test]
  fn bearing_maps_to_eight_sectors() {
    assert_eq!(sector_of_bearing(0.0), 0);
    assert_eq!(sector_of_bearing(22.4), 0);
    assert_eq!(sector_of_bearing(22.6), 1);
    assert_eq!(sector_of_bearing(90.0), 2);
    assert_eq!(sector_of_bearing(180.0), 4);
    assert_eq!(sector_of_bearing(270.0), 6);
    assert_eq!(sector_of_bearing(337.4), 7);
    assert_eq!(sector_of_bearing(337.5), 0);
    assert_eq!(sector_of_bearing(359.9), 0);
    assert_eq!(sector_of_bearing(-10.0), 0);
    assert_eq!(sector_of_bearing(370.0), 0);
  }

  #[test]
  fn sector_centers_are_multiples_of_45() {
    let centers: Vec<f64> = (0..SECTORS).map(sector_center_deg).collect();
    assert_eq!(
      centers,
      [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0]
    );
  }

  #[test]
  fn east_of_beijing_lands_in_the_east_sector() {
    // OM89 = 北京附近；PM95 = 东京附近，几乎正东。
    let spots = vec![spot("PM95", "20m", -10.0, 12)];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.sectors[2].count, 1);
    assert_eq!(coverage.sectors[2].mean_snr, -10.0);
    assert_eq!(coverage.by_band.len(), 1);
    assert_eq!(coverage.by_band[0].0, "20m");
  }

  #[test]
  fn invalid_report_locators_are_skipped() {
    let spots = vec![
      spot("", "20m", 0.0, 3),
      spot("??", "20m", 0.0, 3),
      spot("OM89", "40m", 5.0, 3),
    ];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.by_band.len(), 1);
    assert_eq!(coverage.by_band[0].0, "40m");
  }

  #[test]
  fn invalid_home_locator_has_no_coverage() {
    assert!(aggregate("", &[]).is_none());
    assert!(aggregate("ZZ99", &[]).is_none());
  }

  #[test]
  fn mean_snr_is_per_sector_and_per_band() {
    let spots = vec![
      spot("PM95", "20m", -10.0, 8),
      spot("PM96", "20m", -20.0, 8),
      spot("PM95", "40m", 6.0, 8),
    ];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    // 两条 20m 都落在东扇区，均值 -15。
    let twenty = &coverage
      .by_band
      .iter()
      .find(|(b, _)| b == "20m")
      .expect("20m")
      .1;
    assert_eq!(twenty[2].count, 2);
    assert!((twenty[2].mean_snr + 15.0).abs() < 1e-9);
    // 40m 只有一条，均值就是它自己。
    let forty = &coverage
      .by_band
      .iter()
      .find(|(b, _)| b == "40m")
      .expect("40m")
      .1;
    assert_eq!(forty[2].count, 1);
    assert_eq!(forty[2].mean_snr, 6.0);
  }

  #[test]
  fn bands_are_sorted_by_report_count() {
    let spots = vec![
      spot("PM95", "40m", 0.0, 0),
      spot("PM95", "20m", 0.0, 0),
      spot("PM96", "20m", 0.0, 0),
    ];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    assert_eq!(coverage.by_band[0].0, "20m");
    assert_eq!(coverage.by_band[1].0, "40m");
  }

  #[test]
  fn hourly_histogram_counts_and_wraps() {
    let spots = vec![
      spot("PM95", "20m", 0.0, 8),
      spot("PM95", "20m", 0.0, 8),
      spot("PM95", "40m", 0.0, 19),
      // 25 表示越界值，应折回 1 点。
      spot("PM95", "40m", 0.0, 25),
    ];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    assert_eq!(coverage.hourly[8], 2);
    assert_eq!(coverage.hourly[19], 1);
    assert_eq!(coverage.hourly[1], 1);
    assert_eq!(coverage.hourly.iter().sum::<usize>(), coverage.total);
    assert_eq!(coverage.busiest_hour(), Some(8));
  }

  #[test]
  fn unknown_hours_are_not_bucketed_at_midnight() {
    // 没有时间戳的数据源（RBN）传 `None`：不能靠填 0 蒙混，否则「最忙时段」永远是 0 点。
    let mut unknown = spot("PM95", "20m", 0.0, 0);
    unknown.hour_utc = None;
    let spots = vec![spot("PM95", "20m", 0.0, 8), unknown];
    let coverage = aggregate("OM89", &spots).expect("本台网格有效");
    assert_eq!(coverage.total, 2);
    assert_eq!(coverage.hourly[0], 0);
    assert_eq!(coverage.busiest_hour(), Some(8));
  }

  #[test]
  fn busiest_hour_is_none_without_reports() {
    let coverage = aggregate("OM89", &[]).expect("本台网格有效");
    assert_eq!(coverage.busiest_hour(), None);
  }
}
