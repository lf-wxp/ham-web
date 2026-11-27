//! 这条 DX 值不值得叫：把分散在呼号页、`/muf`、日志和 `/nec` 的事实拼在一起。
//!
//! 只**摆事实、不下结论**：每一项独立给出，缺了所需的输入就留空（由界面说明缺什么），**不合成一个总分**。
//!
//! - **方位与距离**：本台网格 → 该呼号所属 DXCC 实体的中心（粗定位，大国内部误差可达上千公里）；
//! - **该小时的可靠度**：[`voacap::predict_at`] 在这个 UTC 小时的各 HF 波段结果，需要本台网格与太阳黑子数；
//! - **路径与晨昏圈**：沿大圆路径取点，看有多大比例落在晨昏圈 [`GREY_NEAR_KM`] 之内。
//!   「贴着」的口径是**至少一半的路径点**在这个距离内——路径只是横穿一次晨昏圈不算贴着；
//! - **日志里的新实体 / 新波段**：复用 [`dx_watch::Worked`]（`/dx-spots` 用的同一份判定）；
//! - **NEC 设计在该方位的增益**不在这里算（要先解算天线），见 [`crate::antenna_coverage::gain_toward`]。

use crate::dx_watch::{Need, Worked};
use crate::dxcc::{Entity, lookup};
use crate::grid::{distance_bearing, great_circle_path, grid_from_lat_lon, lat_lon_from_grid};
use crate::logbook::LogEntry;
use crate::voacap;
use crate::watch_windows::{BANDS, GREY_NEAR_KM, Path, When, distance_to_terminator_km};

/// 沿路径取多少段（含两端共 `PATH_STEPS + 1` 个点）。
pub const PATH_STEPS: usize = 48;

/// 「贴着晨昏圈」：至少这个比例的路径点落在 [`GREY_NEAR_KM`] 之内。
pub const ALONG_SHARE: f64 = 0.5;

/// 一个 HF 波段在这个钟点的可靠度。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandReliability {
  pub band: &'static str,
  /// 0–1，见 [`voacap::BandResult::reliability`]。
  pub reliability: f64,
  pub usable: bool,
}

/// 路径与晨昏圈的关系。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathTerminator {
  /// 路径点里离晨昏圈最近的距离（km）。
  pub min_km: f64,
  /// 落在 [`GREY_NEAR_KM`] 之内的路径点比例（0–1）。
  pub share_near: f64,
  /// 是否「贴着」：`share_near ≥` [`ALONG_SHARE`]。
  pub along: bool,
}

/// 与日志的关系。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
  /// 该实体从没联过。
  pub new_entity: bool,
  /// 联过该实体、但**没在这些波段**联过（按波段从低到高）。
  pub new_bands: Vec<&'static str>,
  /// 这个呼号本身在这些波段联过。
  pub worked_bands: Vec<&'static str>,
}

/// 汇总的事实。每一项缺输入就是 `None`。
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
  /// 该呼号所属的 DXCC 实体；认不出为 `None`（后面各项都会是 `None`）。
  pub entity: Option<&'static Entity>,
  /// 本台指向实体中心的方位与距离：需要本台网格。
  pub path: Option<Path>,
  /// 这个 UTC 小时各 HF 波段的可靠度：需要本台网格与太阳黑子数。
  pub reliability: Option<Vec<BandReliability>>,
  /// 路径与晨昏圈：需要本台网格。
  pub terminator: Option<PathTerminator>,
  /// 与日志的关系：需要日志里至少有一条通联。
  pub standing: Option<Standing>,
}

/// 沿 `(lat1, lon1)` → `(lat2, lon2)` 的大圆路径，看它与晨昏圈的关系。
#[must_use]
pub fn path_terminator(lat1: f64, lon1: f64, lat2: f64, lon2: f64, when: &When) -> PathTerminator {
  let decl =
    crate::astro::solar_declination(crate::astro::day_of_year(when.year, when.month, when.day));
  let sub_lon = crate::astro::subsolar_longitude(when.utc_hour);
  let points = great_circle_path(lat1, lon1, lat2, lon2, PATH_STEPS);
  let distances: Vec<f64> = points
    .iter()
    .map(|&(lat, lon)| distance_to_terminator_km(lat, lon, decl, sub_lon))
    .collect();
  let near = distances.iter().filter(|d| **d <= GREY_NEAR_KM).count();
  let share_near = near as f64 / distances.len() as f64;
  PathTerminator {
    min_km: distances.iter().copied().fold(f64::INFINITY, f64::min),
    share_near,
    along: share_near >= ALONG_SHARE,
  }
}

/// 汇总一个呼号的事实。
///
/// `home_grid` 是本台网格；`entries` 是日志；`ssn` 是太阳黑子数。呼号认不出实体时，除了 `entity` 以外全是 `None`。
#[must_use]
pub fn facts(
  call: &str,
  home_grid: &str,
  entries: &[LogEntry],
  when: &When,
  ssn: Option<f64>,
) -> Facts {
  let call = call.trim().to_ascii_uppercase();
  let call = call.as_str();
  let entity = lookup(call);
  let Some(e) = entity else {
    return Facts {
      entity: None,
      path: None,
      reliability: None,
      terminator: None,
      standing: None,
    };
  };
  let home = lat_lon_from_grid(home_grid);

  let path = home.map(|(lat, lon)| {
    let (distance_km, bearing_deg) = distance_bearing(lat, lon, e.lat, e.lon);
    Path {
      distance_km,
      bearing_deg,
    }
  });
  let terminator = home.map(|(lat, lon)| path_terminator(lat, lon, e.lat, e.lon, when));

  // 对方网格取实体中心所在的网格（6 位，约 4 × 2 km）。
  let reliability = match (home, ssn, grid_from_lat_lon(e.lat, e.lon)) {
    (Some(_), Some(ssn), Some(entity_grid)) => {
      voacap::predict_at(
        home_grid.trim(),
        &entity_grid,
        when.month,
        ssn,
        Some(when.utc_hour),
      )
      .map(|p| {
        // 只给 HF 十个波段（模型的范围），按频率从低到高；不排序、不挑「最好的」。
        p.bands
          .iter()
          .map(|b| BandReliability {
            band: b.band,
            reliability: b.reliability,
            usable: b.usable,
          })
          .collect()
      })
    }
    _ => None,
  };

  let standing = (!entries.is_empty()).then(|| {
    let worked = Worked::from_entries(entries);
    let needs: Vec<(&'static str, Need)> =
      BANDS.iter().map(|&b| (b, worked.need(call, b))).collect();
    Standing {
      // 实体没联过时，每个波段都是「新实体」。
      new_entity: needs.iter().all(|(_, n)| *n == Need::NewDxcc),
      new_bands: needs
        .iter()
        .filter(|(_, n)| *n == Need::NewBand)
        .map(|(b, _)| *b)
        .collect(),
      worked_bands: needs
        .iter()
        .filter(|(_, n)| *n == Need::Worked)
        .map(|(b, _)| *b)
        .collect(),
    }
  });

  Facts {
    entity,
    path,
    reliability,
    terminator,
    standing,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const NOON: When = When {
    year: 2026,
    month: 10,
    day: 15,
    utc_hour: 12.0,
  };

  fn qso(call: &str, freq: &str) -> LogEntry {
    LogEntry {
      callsign: call.to_owned(),
      freq: freq.to_owned(),
      date: "2026-01-01".to_owned(),
      time: "00:00".to_owned(),
      mode: "FT8".to_owned(),
      ..LogEntry::default()
    }
  }

  // ---------------- 缺输入就留空 ----------------

  #[test]
  fn each_item_is_blank_when_its_own_input_is_missing() {
    let log = [qso("W1AW", "14.074")];
    // 全齐。
    let all = facts("JA1ABC", "OM89", &log, &NOON, Some(100.0));
    assert!(all.entity.is_some() && all.path.is_some() && all.terminator.is_some());
    assert!(all.reliability.is_some() && all.standing.is_some());

    // 缺本台网格：方位、可靠度、晨昏圈都没有；日志部分不受影响。
    for grid in ["", "ZZ99", "!!"] {
      let f = facts("JA1ABC", grid, &log, &NOON, Some(100.0));
      assert!(
        f.path.is_none() && f.reliability.is_none() && f.terminator.is_none(),
        "{grid:?}"
      );
      assert!(f.standing.is_some(), "{grid:?}");
    }
    // 缺太阳黑子数：只有可靠度没有，方位与晨昏圈照给（它们是几何，不依赖太阳数据）。
    let f = facts("JA1ABC", "OM89", &log, &NOON, None);
    assert!(f.reliability.is_none() && f.path.is_some() && f.terminator.is_some());
    // 日志是空的：没有「新实体 / 新波段」可言（空日志里一切都是新的，这个结论没有信息量）。
    let f = facts("JA1ABC", "OM89", &[], &NOON, Some(100.0));
    assert!(f.standing.is_none() && f.path.is_some() && f.reliability.is_some());
  }

  #[test]
  fn an_unrecognised_call_has_nothing_but_the_missing_entity() {
    let log = [qso("W1AW", "14.074")];
    for call in ["", "   ", "???"] {
      let f = facts(call, "OM89", &log, &NOON, Some(100.0));
      assert_eq!(f.entity, None, "{call:?}");
      assert!(
        f.path.is_none()
          && f.reliability.is_none()
          && f.terminator.is_none()
          && f.standing.is_none()
      );
    }
  }

  // ---------------- 方位与距离 ----------------

  #[test]
  fn bearing_and_distance_are_home_to_the_entity_centre() {
    let f = facts("JA1ABC", "OM89", &[], &NOON, None);
    let e = f.entity.unwrap();
    let (la, lo) = lat_lon_from_grid("OM89").unwrap();
    let (km, br) = distance_bearing(la, lo, e.lat, e.lon);
    let p = f.path.unwrap();
    assert!((p.distance_km - km).abs() < 1e-9 && (p.bearing_deg - br).abs() < 1e-9);
    // 与呼号页原有的「方位 / 距离」同一个数：不另算一套。
    let old = crate::callsign_insight::insight("JA1ABC", "OM89", &[]);
    assert_eq!(old.azimuth_deg, Some(p.bearing_deg));
    assert_eq!(old.distance_km, Some(p.distance_km));
    // 北京到日本：向东偏南、两三千公里。
    assert!(
      (70.0..120.0).contains(&p.bearing_deg) && (1500.0..3500.0).contains(&p.distance_km),
      "{p:?}"
    );
  }

  // ---------------- 可靠度 ----------------

  #[test]
  fn reliability_is_the_voacap_result_for_this_hour_and_only_hf() {
    let f = facts("VK2XX", "OM89", &[], &NOON, Some(100.0));
    let rel = f.reliability.unwrap();
    assert_eq!(rel.len(), 10, "只给 HF 十个波段");
    assert_eq!(
      rel.iter().map(|b| b.band).collect::<Vec<_>>(),
      voacap::HF_BANDS.iter().map(|(b, _)| *b).collect::<Vec<_>>(),
      "按频率从低到高、不排序"
    );
    // 与模型逐位一致：对方网格是实体中心所在的网格。
    let e = f.entity.unwrap();
    let grid = grid_from_lat_lon(e.lat, e.lon).unwrap();
    let p = voacap::predict_at("OM89", &grid, 10, 100.0, Some(12.0)).unwrap();
    for (got, want) in rel.iter().zip(&p.bands) {
      assert_eq!(
        (got.reliability, got.usable),
        (want.reliability, want.usable),
        "{}",
        got.band
      );
    }
    // 小时与黑子数确实起作用：黑子数 0、深夜时 10 m 不通。
    let night = facts(
      "VK2XX",
      "OM89",
      &[],
      &When {
        utc_hour: 2.0,
        ..NOON
      },
      Some(0.0),
    );
    let ten = night
      .reliability
      .unwrap()
      .into_iter()
      .find(|b| b.band == "10m")
      .unwrap();
    assert!(!ten.usable && ten.reliability == 0.0, "{ten:?}");
  }

  // ---------------- 晨昏圈 ----------------

  /// 春分日、UTC 12:00：直射点经度 0，晨昏圈是经线 ±90°（赤纬 ≈ −0.4°，偏差不到 50 km）。
  const EQUINOX: When = When {
    year: 2026,
    month: 3,
    day: 21,
    utc_hour: 12.0,
  };

  #[test]
  fn a_path_running_along_the_terminator_is_flagged() {
    // 沿东经 90° 从 60°S 走到 60°N：几乎整条都在晨昏圈上。
    let t = path_terminator(-60.0, 90.0, 60.0, 90.0, &EQUINOX);
    assert!(t.along && t.share_near > 0.95, "{t:?}");
    assert!(t.min_km < 100.0, "{t:?}");
  }

  #[test]
  fn a_path_crossing_the_terminator_once_is_not_flagged() {
    // 沿赤道从东经 0° 走到东经 180°：恰好横穿东经 90° 的晨昏圈一次，其余远离它。
    let t = path_terminator(0.0, 0.0, 0.0, 179.0, &EQUINOX);
    assert!(!t.along, "{t:?}");
    assert!(t.share_near < 0.4 && t.share_near > 0.0, "{t:?}");
    assert!(t.min_km < 200.0, "确实穿过了晨昏圈：{t:?}");
  }

  #[test]
  fn a_path_wholly_in_daylight_is_far_from_the_terminator() {
    // 正午的直射点附近的短路径。
    let t = path_terminator(0.0, -10.0, 0.0, 10.0, &EQUINOX);
    assert!(
      !t.along && t.share_near == 0.0 && t.min_km > 7000.0,
      "{t:?}"
    );
  }

  #[test]
  fn share_near_is_a_fraction_over_all_path_points() {
    let t = path_terminator(-60.0, 90.0, 60.0, 90.0, &EQUINOX);
    assert!((0.0..=1.0).contains(&t.share_near));
    // 比例是 `k / (PATH_STEPS + 1)` 的整数倍。
    let k = t.share_near * (PATH_STEPS + 1) as f64;
    assert!((k - k.round()).abs() < 1e-9, "{k}");
  }

  // ---------------- 日志 ----------------

  #[test]
  fn new_entity_new_band_and_worked_follow_the_log() {
    // 日志：20 m 联过 JA（另一个呼号）、40 m 联过 JA1ABC 本人、15 m 联过 W。
    let log = [
      qso("JA2XYZ", "14.074"),
      qso("JA1ABC", "7.074"),
      qso("W1AW", "21.074"),
    ];
    // JA1ABC：实体联过，不是新实体；20 m 联过同实体（别的呼号）所以不算新波段；40 m 联过本人。
    let s = facts("JA1ABC", "OM89", &log, &NOON, None).standing.unwrap();
    assert!(!s.new_entity);
    assert_eq!(s.worked_bands, ["40m"]);
    // 新波段 = 联过实体、但没在这个波段联过：除了 20 m（JA2XYZ）与 40 m（本人）的其余波段。
    let expect: Vec<&str> = BANDS
      .iter()
      .copied()
      .filter(|b| !["20m", "40m"].contains(b))
      .collect();
    assert_eq!(s.new_bands, expect);

    // 一个没联过的实体：新实体，没有「新波段」「联过的波段」可说。
    let s = facts("VK2XX", "OM89", &log, &NOON, None).standing.unwrap();
    assert!(
      s.new_entity && s.new_bands.is_empty() && s.worked_bands.is_empty(),
      "{s:?}"
    );
  }

  #[test]
  fn worked_on_every_band_leaves_nothing_new() {
    let log: Vec<LogEntry> = [
      "1.84", "3.57", "5.358", "7.074", "10.136", "14.074", "18.1", "21.074", "24.915", "28.074",
      "50.313", "144.174", "432.1",
    ]
    .iter()
    .map(|f| qso("JA1ABC", f))
    .collect();
    let s = facts("JA1ABC", "OM89", &log, &NOON, None).standing.unwrap();
    assert!(!s.new_entity && s.new_bands.is_empty());
    assert_eq!(s.worked_bands, BANDS, "十三个波段都联过本人");
  }

  #[test]
  fn call_matching_ignores_case_and_spaces() {
    let log = [qso("JA1ABC", "14.074")];
    let s = facts(" ja1abc ", "OM89", &log, &NOON, None)
      .standing
      .unwrap();
    assert_eq!(s.worked_bands, ["20m"]);
  }
}
