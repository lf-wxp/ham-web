//! 现在守哪一段：这个钟点、这张操作证，该守哪几个波段窗口。
//!
//! 执照类别、波段规划、点对点可靠度、灰线几何站内各自都能算，没有一处把它们放在一起回答这个问题。
//! 本模块只**串联**，不造新的传播模型：
//!
//! - **窗口** = 波段规划里的波段，经 [`license_compliance::check`] 按类别过滤。**证不允许的窗口不出现**
//!   （A 类只有 30 MHz 以上，所以 HF 一个都没有）；
//! - **可靠度** 只在同时有「本台网格 + 对方网格 + 太阳黑子数」时才附上，来自
//!   [`voacap::predict_at`] 在这个 UTC 小时的结果；缺任何一样就只列允许的窗口，**不编「好 / 差」**。
//!   VHF / UHF 没有 HF 的电离层模型，同样不给可靠度；
//! - **灰线** 只是几何事实：本台到晨昏圈的大圆距离，以及落在黎明侧还是黄昏侧。
//!   「灰线附近低频段常有增强」是业余无线电的经验说法，不是本模块算出来的结论，所以不写进结果里。
//!
//! 地磁（K ≥ 4）**不折减** MUF：沿用站内现有的传播等级提示（[`crate::solar::live_level`]），
//! 本模块不接收 K。

use crate::astro;
use crate::bandplan::{BAND_PLANS, SegmentModes, parse_range};
use crate::grid::{distance_bearing, lat_lon_from_grid};
use crate::license_compliance::{Class, check};
use crate::voacap;

/// 候选窗口：波段规划里的 HF 波段加 6 m / 2 m / 70 cm，按频率从低到高。
///
/// 不含 2200 m / 630 m（LF / MF 新开放段，没有对应的电离层模型）与微波段。
pub const BANDS: [&str; 13] = [
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

/// 「贴近晨昏圈」的距离阈值（km）：约赤道上日出 / 日落前后 55 分钟（15° 经度 = 1 小时 ≈ 1668 km）。
/// 这是一个**显示用的口径**，不是传播结论。
pub const GREY_NEAR_KM: f64 = 1500.0;

/// 地球上 1° 大圆弧长（km，R = 6371）。
const KM_PER_DEG: f64 = 111.194_926_644_558_73;

/// 哪一刻。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct When {
  pub year: u32,
  pub month: u32,
  pub day: u32,
  /// UTC 小时（0–24，可带小数）。
  pub utc_hour: f64,
}

/// 一个窗口在这个钟点的预测。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Forecast {
  /// 相对可靠度（0–1），见 [`voacap::BandResult::reliability`]。
  pub reliability: f64,
  /// 参考频率是否落在 LUF–MUF 之间。
  pub usable: bool,
}

/// 一个允许守的窗口。
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
  pub band: &'static str,
  /// 这张操作证在这个波段的**发射功率**上限（W，已与 ITU 脚注取较小者）。
  pub power_w: f64,
  /// 是否需要开展频率协调（次要业务或与其他业务共同主要）。
  pub needs_coordination: bool,
  /// 波段规划里这个波段有哪些模式子段（电报 / 窄带数据 / 话务的并集）。
  pub modes: SegmentModes,
  /// 该小时的预测；没有双方网格 / 太阳数据、或不是 HF 波段时为 `None`。
  pub forecast: Option<Forecast>,
}

/// 灰线落在哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreyKind {
  /// 黎明侧（本台在太阳直射点以西：地方时在正午之前）。
  Dawn,
  /// 黄昏侧（本台在太阳直射点以东：地方时在正午之后）。
  Dusk,
}

/// 本台与晨昏圈的几何关系。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GreyLine {
  pub kind: GreyKind,
  /// 本台到晨昏圈的大圆距离（km）。
  pub distance_km: f64,
  /// 是否在 [`GREY_NEAR_KM`] 之内。
  pub near: bool,
}

/// 两端的大圆距离与方位。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Path {
  pub distance_km: f64,
  pub bearing_deg: f64,
}

/// 整份结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Watch {
  pub windows: Vec<Window>,
  /// 窗口是否按可靠度排过序（只有拿到预测才排）。
  pub ranked: bool,
  /// 本台网格合法时才有。
  pub grey_line: Option<GreyLine>,
  /// 双方网格都合法时才有。
  pub path: Option<Path>,
}

/// 某点到晨昏圈的大圆距离（km）。
///
/// 晨昏圈是以太阳直射点为极点的大圆：离直射点的角距离 `D` 恰好 90°。所以到晨昏圈的距离就是
/// `|D − 90°|` 乘每度弧长。`D` 由球面余弦定理求（直射点纬度 = 赤纬，经度 = `subsolar_lon`）。
#[must_use]
pub fn distance_to_terminator_km(lat: f64, lon: f64, decl: f64, subsolar_lon: f64) -> f64 {
  let (phi, delta) = (lat.to_radians(), decl.to_radians());
  let dl = (lon - subsolar_lon).to_radians();
  let d = (phi.sin() * delta.sin() + phi.cos() * delta.cos() * dl.cos())
    .clamp(-1.0, 1.0)
    .acos()
    .to_degrees();
  (d - 90.0).abs() * KM_PER_DEG
}

/// 本台此刻与晨昏圈的关系。
#[must_use]
pub fn grey_line(lat: f64, lon: f64, when: &When) -> GreyLine {
  let decl = astro::solar_declination(astro::day_of_year(when.year, when.month, when.day));
  let sub_lon = astro::subsolar_longitude(when.utc_hour);
  let distance_km = distance_to_terminator_km(lat, lon, decl, sub_lon);
  // 把经度差折到 (−180, 180]：> 0 在直射点以东（地方时过了正午 → 黄昏侧），≤ 0 在以西（黎明侧）。
  let h = (lon - sub_lon + 540.0).rem_euclid(360.0) - 180.0;
  GreyLine {
    kind: if h > 0.0 {
      GreyKind::Dusk
    } else {
      GreyKind::Dawn
    },
    distance_km,
    near: distance_km <= GREY_NEAR_KM,
  }
}

/// 波段规划里这个波段的 `(参考频率 MHz, 模式并集)`；规划里没有就是 `None`。
fn plan_of(band: &str) -> Option<(f64, SegmentModes)> {
  let plan = BAND_PLANS.iter().find(|p| p.band == band)?;
  let (lo, hi) = parse_range(plan.freq_range)?;
  let mut modes = SegmentModes {
    cw: false,
    narrow: false,
    phone: false,
  };
  for seg in plan.segments {
    modes.cw |= seg.modes.cw;
    modes.narrow |= seg.modes.narrow;
    modes.phone |= seg.modes.phone;
  }
  Some(((lo + hi) / 2.0, modes))
}

/// 给定操作证类别，列出这个钟点能守的窗口。
///
/// `home_grid` / `target_grid` 是 Maidenhead 网格（4 位或 6 位）；`ssn` 是太阳黑子数。
/// 三样齐全才附可靠度并按可靠度降序排列（同分保持频率从低到高）；否则窗口按频率从低到高、没有预测。
#[must_use]
pub fn watch(
  class: Class,
  home_grid: &str,
  target_grid: &str,
  when: &When,
  ssn: Option<f64>,
) -> Watch {
  let home = lat_lon_from_grid(home_grid);
  let target = lat_lon_from_grid(target_grid);
  let prediction = match (home, target, ssn) {
    (Some(_), Some(_), Some(ssn)) => voacap::predict_at(
      home_grid.trim(),
      target_grid.trim(),
      when.month,
      ssn,
      Some(when.utc_hour),
    ),
    _ => None,
  };

  let mut windows: Vec<Window> = BANDS
    .iter()
    .filter_map(|&band| {
      let (mid, modes) = plan_of(band)?;
      let verdict = check(class, mid);
      // 证不允许的窗口不出现。
      if !verdict.class_permitted {
        return None;
      }
      let forecast = prediction.as_ref().and_then(|p| {
        p.bands.iter().find(|b| b.band == band).map(|b| Forecast {
          reliability: b.reliability,
          usable: b.usable,
        })
      });
      Some(Window {
        band,
        power_w: verdict.tx_power_limit_w?,
        needs_coordination: verdict.needs_coordination,
        modes,
        forecast,
      })
    })
    .collect();

  let ranked = prediction.is_some();
  if ranked {
    // 稳定排序：没有预测的（VHF / UHF）排在有预测的后面，且彼此保持频率顺序。
    windows.sort_by(|a, b| {
      let ra = a.forecast.map(|f| f.reliability);
      let rb = b.forecast.map(|f| f.reliability);
      match (ra, rb) {
        (Some(x), Some(y)) => y.total_cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
      }
    });
  }

  Watch {
    windows,
    ranked,
    grey_line: home.map(|(lat, lon)| grey_line(lat, lon, when)),
    // 路径是纯几何（两端网格中心的大圆距离与方位），只依赖两个网格，不依赖太阳数据：
    // 缺太阳黑子数时可靠度留空，但方位 / 距离照给（与 `dx_facts` 的 `path` 同一口径）。
    path: match (home, target) {
      (Some((lat1, lon1)), Some((lat2, lon2))) => {
        let (distance_km, bearing_deg) = distance_bearing(lat1, lon1, lat2, lon2);
        Some(Path {
          distance_km,
          bearing_deg,
        })
      }
      _ => None,
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::grid::distance_bearing;

  const OCT_NOON: When = When {
    year: 2026,
    month: 10,
    day: 15,
    utc_hour: 12.0,
  };

  fn names(w: &Watch) -> Vec<&'static str> {
    w.windows.iter().map(|x| x.band).collect()
  }

  // ---------------- 候选窗口自洽 ----------------

  #[test]
  fn every_candidate_has_a_plan_and_an_amateur_allocation() {
    for band in BANDS {
      let (mid, modes) = plan_of(band).unwrap_or_else(|| panic!("{band} 不在波段规划里"));
      // B / C 类没有频率范围限制：能通过说明这个参考频率落在某个业余划分里。
      for class in [Class::B, Class::C] {
        let c = check(class, mid);
        assert!(c.allocation.is_some(), "{band} {mid} MHz 不在业余划分里");
        assert!(c.class_permitted, "{band}");
      }
      assert!(
        modes.cw || modes.narrow || modes.phone,
        "{band} 规划里没有任何模式"
      );
    }
    // 候选按频率从低到高排列，没有重复。
    let freqs: Vec<f64> = BANDS.iter().map(|b| plan_of(b).unwrap().0).collect();
    assert!(freqs.windows(2).all(|w| w[0] < w[1]), "{freqs:?}");
  }

  // ---------------- 证外不出现 ----------------

  #[test]
  fn class_a_only_gets_the_vhf_and_uhf_windows() {
    // A 类只有 30–3000 MHz：HF 的十个窗口一个都不该出现。
    let w = watch(Class::A, "", "", &OCT_NOON, None);
    assert_eq!(names(&w), ["6m", "2m", "70cm"]);
    assert!(w.windows.iter().all(|x| x.power_w == 25.0), "A 类 25 W");
  }

  #[test]
  fn class_b_and_c_get_every_window_with_their_own_power_limits() {
    for class in [Class::B, Class::C] {
      let w = watch(class, "", "", &OCT_NOON, None);
      assert_eq!(names(&w), BANDS, "{class:?}");
    }
    let power = |class, band: &str| {
      watch(class, "", "", &OCT_NOON, None)
        .windows
        .iter()
        .find(|x| x.band == band)
        .unwrap()
        .power_w
    };
    // 办法第三十条：B 类 30 MHz 以下 < 15 W、以上 ≤ 25 W；C 类 30 MHz 以下 ≤ 1000 W、以上 ≤ 25 W。
    assert_eq!(power(Class::B, "20m"), 15.0);
    assert_eq!(power(Class::B, "2m"), 25.0);
    assert_eq!(power(Class::C, "20m"), 1000.0);
    assert_eq!(power(Class::C, "70cm"), 25.0);
    // 60 m 有 ITU 脚注 5.133B 的 15 W e.i.r.p. 上限：C 类也不是 1000 W。
    assert_eq!(power(Class::C, "60m"), 15.0);
    assert_eq!(power(Class::B, "60m"), 15.0);
  }

  #[test]
  fn coordination_and_plan_modes_come_from_the_existing_tables() {
    let w = watch(Class::C, "", "", &OCT_NOON, None);
    let get = |b: &str| w.windows.iter().find(|x| x.band == b).unwrap();
    // 160 m 规划里有 CW、窄带数据与话音三段（1.800–1.830 / 1.830–1.840 / 1.840–2.000）。
    let m = get("160m").modes;
    assert!(m.cw && m.narrow && m.phone);
    // 与 `license_compliance::check` 同一口径：不另算「要不要协调」。
    for win in &w.windows {
      let (mid, _) = plan_of(win.band).unwrap();
      assert_eq!(
        win.needs_coordination,
        check(Class::C, mid).needs_coordination,
        "{}",
        win.band
      );
    }
  }

  // ---------------- 不编「好 / 差」 ----------------

  #[test]
  fn without_grids_or_solar_data_no_window_gets_a_forecast_or_a_rank() {
    for (home, target, ssn) in [
      ("", "", None),
      ("OM89", "", Some(100.0)),     // 缺对方网格
      ("", "IO91", Some(100.0)),     // 缺本台网格
      ("OM89", "IO91", None),        // 缺太阳数据
      ("ZZ99", "IO91", Some(100.0)), // 本台网格非法
      ("OM89", "!!", Some(100.0)),   // 对方网格非法
    ] {
      let w = watch(Class::C, home, target, &OCT_NOON, ssn);
      assert!(!w.ranked, "{home:?} {target:?} {ssn:?}");
      assert!(
        w.windows.iter().all(|x| x.forecast.is_none()),
        "{home:?} {target:?} {ssn:?}"
      );
      assert_eq!(names(&w), BANDS, "没有预测时按频率从低到高");
      // 路径是纯几何：只看两个网格合不合法，与太阳数据无关（缺太阳黑子数仍有路径）。
      assert_eq!(
        w.path.is_some(),
        lat_lon_from_grid(home).is_some() && lat_lon_from_grid(target).is_some(),
        "{home:?} {target:?}"
      );
    }
  }

  #[test]
  fn forecast_matches_the_voacap_model_and_ranks_by_reliability() {
    let w = watch(Class::C, "OM89", "IO91", &OCT_NOON, Some(100.0));
    assert!(w.ranked);
    let p = voacap::predict_at("OM89", "IO91", 10, 100.0, Some(12.0)).unwrap();
    for win in &w.windows {
      match p.bands.iter().find(|b| b.band == win.band) {
        // HF：与模型逐位一致，不另算一套。
        Some(b) => {
          let f = win
            .forecast
            .unwrap_or_else(|| panic!("{} 应有预测", win.band));
          assert_eq!(
            (f.reliability, f.usable),
            (b.reliability, b.usable),
            "{}",
            win.band
          );
        }
        // VHF / UHF：没有电离层模型，不编可靠度。
        None => assert!(win.forecast.is_none(), "{} 不该有预测", win.band),
      }
    }
    // 有预测的在前、按可靠度降序；没有预测的（6 m / 2 m / 70 cm）殿后且保持频率顺序。
    let with: Vec<f64> = w
      .windows
      .iter()
      .filter_map(|x| x.forecast.map(|f| f.reliability))
      .collect();
    assert_eq!(with.len(), 10);
    assert!(with.windows(2).all(|p| p[0] >= p[1]), "{with:?}");
    assert_eq!(names(&w)[10..], ["6m", "2m", "70cm"]);
    // 路径的距离与方位就是两个网格中心的大圆。
    let (la, lo) = lat_lon_from_grid("OM89").unwrap();
    let (lb, lob) = lat_lon_from_grid("IO91").unwrap();
    let (km, br) = distance_bearing(la, lo, lb, lob);
    let path = w.path.unwrap();
    assert!((path.distance_km - km).abs() < 1e-6 && (path.bearing_deg - br).abs() < 1e-6);
  }

  #[test]
  fn the_forecast_follows_the_hour_and_the_sunspot_number() {
    // 北京 → 伦敦：太阳黑子数 0 时 10 m 过不去；黑子数 200、白天的 20 m 总有可靠度。
    let quiet = watch(
      Class::C,
      "OM89",
      "IO91",
      &When {
        utc_hour: 2.0,
        ..OCT_NOON
      },
      Some(0.0),
    );
    let ten = quiet
      .windows
      .iter()
      .find(|x| x.band == "10m")
      .unwrap()
      .forecast
      .unwrap();
    assert!(!ten.usable && ten.reliability == 0.0, "{ten:?}");
    let busy = watch(Class::C, "OM89", "IO91", &OCT_NOON, Some(200.0));
    let twenty = busy
      .windows
      .iter()
      .find(|x| x.band == "20m")
      .unwrap()
      .forecast
      .unwrap();
    assert!(twenty.usable && twenty.reliability > 0.0, "{twenty:?}");
  }

  #[test]
  fn a_geomagnetic_storm_is_not_a_parameter_so_it_cannot_discount_the_muf() {
    // `watch` 不接收 K：同样的输入只有一个结果。K ≥ 4 的提示沿用 `solar::live_level`，不在这里折减。
    let a = watch(Class::C, "OM89", "IO91", &OCT_NOON, Some(100.0));
    let b = watch(Class::C, "OM89", "IO91", &OCT_NOON, Some(100.0));
    assert_eq!(a, b);
  }

  // ---------------- 灰线几何 ----------------

  #[test]
  fn terminator_distance_matches_hand_computed_geometry() {
    // 太阳直射赤道、经度 0（春秋分正午）：晨昏圈是经线 ±90°。
    assert!(distance_to_terminator_km(0.0, 90.0, 0.0, 0.0) < 1e-6);
    assert!(distance_to_terminator_km(0.0, -90.0, 0.0, 0.0) < 1e-6);
    // 直射点正下方离晨昏圈 90°；北纬 45° 的正午点离直射点 45°，离晨昏圈 45°。
    assert!((distance_to_terminator_km(0.0, 0.0, 0.0, 0.0) - 90.0 * KM_PER_DEG).abs() < 1e-6);
    assert!((distance_to_terminator_km(45.0, 0.0, 0.0, 0.0) - 45.0 * KM_PER_DEG).abs() < 1e-6);
    // 夏至（赤纬 23.44°）：北纬 66.56° 的正午点离直射点 43.12°，离晨昏圈 46.88°。
    let d = distance_to_terminator_km(66.56, 0.0, 23.44, 0.0);
    assert!((d - (90.0 - 43.12) * KM_PER_DEG).abs() < 1e-6, "{d}");
    // 对跖点（夜半球正中）离晨昏圈也是 90°。
    assert!((distance_to_terminator_km(0.0, 180.0, 0.0, 0.0) - 90.0 * KM_PER_DEG).abs() < 1e-6);
  }

  #[test]
  fn terminator_distance_agrees_with_the_existing_terminator_latitude() {
    // `astro::terminator_lat` 给出某经度处晨昏圈的纬度：那一点到晨昏圈的距离必须是 0。
    // 两个函数各自独立推导，互相核对。赤纬取非零（`terminator_lat` 在赤纬 ≈ 0 时会限幅）。
    for decl in [-23.0, -10.0, 5.0, 15.0, 23.0] {
      for sub_lon in [-150.0, -30.0, 0.0, 75.0, 170.0] {
        for lon in (-180..180).step_by(23) {
          let lon = f64::from(lon);
          let lat = astro::terminator_lat(lon, decl, sub_lon);
          let d = distance_to_terminator_km(lat, lon, decl, sub_lon);
          assert!(d < 1e-6, "decl {decl} sub_lon {sub_lon} lon {lon}: {d} km");
        }
      }
    }
  }

  #[test]
  fn dawn_and_dusk_sides_follow_the_local_solar_time() {
    // 春分附近、UTC 12:00：直射点在经度 0。东经 90° 的地方时是 18:00（黄昏），西经 90° 是 06:00（黎明）。
    let equinox = When {
      year: 2026,
      month: 3,
      day: 21,
      utc_hour: 12.0,
    };
    let dusk = grey_line(0.0, 90.0, &equinox);
    let dawn = grey_line(0.0, -90.0, &equinox);
    assert_eq!((dusk.kind, dawn.kind), (GreyKind::Dusk, GreyKind::Dawn));
    // 都在晨昏圈上（春分的赤纬不是严格的 0，给几十公里的容差）。
    assert!(
      dusk.distance_km < 100.0 && dawn.distance_km < 100.0,
      "{dusk:?} {dawn:?}"
    );
    assert!(dusk.near && dawn.near);
    // 正午时分的本台离晨昏圈很远，不算贴近。
    let noon = grey_line(0.0, 0.0, &equinox);
    assert!(!noon.near && noon.distance_km > 9000.0, "{noon:?}");
  }

  #[test]
  fn near_is_a_closed_threshold_on_distance() {
    // 沿赤道从晨昏圈向正午方向走：离 1500 km 内算贴近，超过不算。
    let w = When {
      year: 2026,
      month: 3,
      day: 21,
      utc_hour: 12.0,
    };
    let base = grey_line(0.0, 90.0, &w).distance_km;
    let step = |km: f64| grey_line(0.0, 90.0 - km / KM_PER_DEG, &w);
    assert!(step(1400.0).near, "{:?}", step(1400.0));
    assert!(!step(1600.0).near, "{:?} base {base}", step(1600.0));
  }

  #[test]
  fn the_grey_line_needs_only_the_home_grid() {
    let w = watch(Class::C, "OM89", "", &OCT_NOON, None);
    assert!(w.grey_line.is_some());
    assert_eq!(w.path, None);
    assert!(watch(Class::C, "", "", &OCT_NOON, None).grey_line.is_none());
    assert!(
      watch(Class::C, "ZZ99", "", &OCT_NOON, None)
        .grey_line
        .is_none()
    );
  }
}
