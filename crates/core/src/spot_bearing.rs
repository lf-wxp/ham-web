//! DX spot 的方位与距离（6 m 开口用）：从本台指向 DX 台的大圆方位。
//!
//! DX cluster 的 spot 没有单独的网格字段，所以方向是**粗定位**：
//!
//! - 默认取 DXCC 实体中心：大国内部的误差可达上千公里；
//! - 备注里**恰好有一个**网格才改用它 —— 两个以上（`FN41<F2>EM63` 这类「两端各一个」的写法）分不清
//!   哪个是 DX 台、哪个是报告者，**不猜**，退回实体中心；没有网格同样退回实体中心；
//! - 本台网格不合法 / 实体也查不到时返回 `None`，由界面说明原因，不编方向。
//!
//! 单个网格也可能是报告者自己的位置而不是 DX 台的，界面因此把依据写明（「按备注里的网格」）。

use crate::dxcc;
use crate::grid::{distance_bearing, lat_lon_from_grid};
use crate::spot_coverage::{SECTORS, sector_of_bearing};

/// 6 m 业余波段（kHz，含两端）：与 [`crate::dx_watch::band_of_khz`] 的 `6m` 口径一致。
pub const SIX_METERS_KHZ: std::ops::RangeInclusive<u32> = 50_000..=54_000;

/// 这个频率是不是 6 m。
#[must_use]
pub fn is_six_meters(khz: u32) -> bool {
  SIX_METERS_KHZ.contains(&khz)
}

/// 方位的依据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Basis {
  /// 备注里恰好有一个网格（已转大写）。
  Grid(String),
  /// DXCC 实体中心（粗定位）。
  Entity,
}

/// 一条 spot 的方位与距离。
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
  /// 初始方位角（度，正北 0、顺时针）。
  pub bearing_deg: f64,
  /// 大圆距离（km）。
  pub distance_km: f64,
  /// 终点从哪来。
  pub basis: Basis,
}

/// 备注里的 Maidenhead 网格：整词匹配 4 位（`FN41`）或 6 位（`EM63KT`），去重、转大写、保持出现顺序。
///
/// `RR73` 是 FT8 的结束报文、不是网格，但恰好符合 4 位网格的格式（RR 是合法的 field），
/// 6 m 的 FT8 备注里很常见，必须排除。
#[must_use]
pub fn grids_in(comment: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  for word in comment.split(|c: char| !c.is_ascii_alphanumeric()) {
    let w = word.to_ascii_uppercase();
    if is_grid(&w) && w != "RR73" && !out.contains(&w) {
      out.push(w);
    }
  }
  out
}

fn is_grid(w: &str) -> bool {
  let b = w.as_bytes();
  if b.len() != 4 && b.len() != 6 {
    return false;
  }
  let field = |c: u8| (b'A'..=b'R').contains(&c);
  let digit = |c: u8| c.is_ascii_digit();
  let sub = |c: u8| (b'A'..=b'X').contains(&c);
  field(b[0])
    && field(b[1])
    && digit(b[2])
    && digit(b[3])
    && (b.len() == 4 || (sub(b[4]) && sub(b[5])))
}

/// 从本台（`home` 为 `(纬度, 经度)`）指向这条 spot 的方位。
///
/// 备注里恰好一个网格 → 用它；否则 → DXCC 实体中心；两者都没有 → `None`。
#[must_use]
pub fn heading(home: (f64, f64), dx_call: &str, comment: &str) -> Option<Heading> {
  let grids = grids_in(comment);
  let (target, basis) = match grids.as_slice() {
    [only] => match lat_lon_from_grid(only) {
      Some(p) => (p, Basis::Grid(only.clone())),
      None => entity_centre(dx_call)?,
    },
    _ => entity_centre(dx_call)?,
  };
  let (distance_km, bearing_deg) = distance_bearing(home.0, home.1, target.0, target.1);
  Some(Heading {
    bearing_deg,
    distance_km,
    basis,
  })
}

fn entity_centre(dx_call: &str) -> Option<((f64, f64), Basis)> {
  let e = dxcc::lookup(dx_call)?;
  Some(((e.lat, e.lon), Basis::Entity))
}

/// 八方位字母（国际通用，不随界面语言变）。
#[must_use]
pub fn compass8(bearing_deg: f64) -> &'static str {
  const NAMES: [&str; SECTORS] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
  NAMES[sector_of_bearing(bearing_deg)]
}

/// 各八方位扇区里有多少条 spot（扇区号同 [`sector_of_bearing`]）。
#[must_use]
pub fn sector_counts(bearings: impl IntoIterator<Item = f64>) -> [usize; SECTORS] {
  let mut counts = [0; SECTORS];
  for b in bearings {
    if b.is_finite() {
      counts[sector_of_bearing(b)] += 1;
    }
  }
  counts
}

#[cfg(test)]
mod tests {
  use super::*;

  // 参考值由独立脚本按 Maidenhead 网格中心点 + haversine（R = 6371 km）算出，不经过被测代码：
  // FN31 中心 (41.5, −73.0) → IO91 中心 (51.5, −1.0)：5392.7 km、52.2°。
  const FN31: (f64, f64) = (41.5, -73.0);

  #[test]
  fn a_single_grid_in_the_comment_wins_over_the_entity_centre() {
    let h = heading(FN31, "G4ABC", "CQ DX IO91").unwrap();
    assert_eq!(h.basis, Basis::Grid("IO91".into()));
    assert!((h.distance_km - 5392.7).abs() < 0.5, "{h:?}");
    assert!((h.bearing_deg - 52.2).abs() < 0.1, "{h:?}");
  }

  #[test]
  fn six_character_grids_and_lowercase_are_read() {
    assert_eq!(grids_in("beacon em63kt up"), ["EM63KT"]);
    assert_eq!(grids_in("fn31 fn31"), ["FN31"], "同一个网格只算一次");
  }

  #[test]
  fn two_grids_are_ambiguous_and_fall_back_to_the_entity() {
    // 「FN41<F2>EM63」是两端各一个，哪个是 DX 台分不出来：不猜。
    let h = heading(FN31, "JA1ABC", "FN41<F2>EM63").unwrap();
    assert_eq!(h.basis, Basis::Entity);
    assert_eq!(grids_in("FN41<F2>EM63"), ["FN41", "EM63"]);
  }

  #[test]
  fn no_grid_uses_the_entity_centre_the_same_way_rbn_does() {
    let h = heading((39.5, 117.0), "JA1ABC", "FT8 -12 dB").unwrap();
    assert_eq!(h.basis, Basis::Entity);
    // 与「实体中心 + distance_bearing」逐位一致：不另写一套算法。
    let e = dxcc::lookup("JA1ABC").unwrap();
    let (d, b) = distance_bearing(39.5, 117.0, e.lat, e.lon);
    assert_eq!((h.distance_km, h.bearing_deg), (d, b));
    // 北京附近到日本：向东偏南、两三千公里。
    assert!((70.0..120.0).contains(&h.bearing_deg), "{h:?}");
    assert!((1500.0..3500.0).contains(&h.distance_km), "{h:?}");
  }

  #[test]
  fn ft8_rr73_is_not_a_grid() {
    // RR73 满足「两个 A–R 字母 + 两个数字」，但它是 FT8 的结束报文。
    assert!(grids_in("FT8 RR73").is_empty());
    assert!(grids_in("rr73").is_empty());
    // 紧挨着一个真网格时，真网格照样读到。
    assert_eq!(grids_in("RR73 IO91"), ["IO91"]);
    // 备注里只有 RR73：按实体中心算，不能把它当成南极网格。
    let h = heading(FN31, "JA1ABC", "RR73").unwrap();
    assert_eq!(h.basis, Basis::Entity);
  }

  #[test]
  fn near_miss_tokens_are_not_grids() {
    for s in [
      "ZZ99", "FN3", "FN311", "FN31ZZ", "FN31Y1", "1N31", "FT8", "JT65", "PSK31", "FN-31",
    ] {
      assert!(grids_in(s).is_empty(), "{s}");
    }
    // 子方块范围是 A–X。
    assert_eq!(grids_in("FN31XX"), ["FN31XX"]);
    // 夹在其它词里不算：整词匹配。
    assert!(grids_in("XFN31X").is_empty());
  }

  #[test]
  fn unknowns_return_none_instead_of_inventing_a_direction() {
    // 实体查不到 + 备注没有网格。
    assert_eq!(heading(FN31, "", "no grid"), None);
    assert_eq!(heading(FN31, "!!!", ""), None);
  }

  #[test]
  fn six_meters_range_matches_the_band_table() {
    assert!(is_six_meters(50_313));
    assert!(is_six_meters(50_000) && is_six_meters(54_000));
    assert!(
      !is_six_meters(49_999)
        && !is_six_meters(54_001)
        && !is_six_meters(144_174)
        && !is_six_meters(0)
    );
    // 与波段表同一口径：每一个 kHz 两边的结论一致。
    for khz in [0, 14_074, 49_999, 50_000, 50_313, 54_000, 54_001, 144_174] {
      assert_eq!(
        is_six_meters(khz),
        crate::dx_watch::band_of_khz(khz) == "6m",
        "{khz}"
      );
    }
  }

  #[test]
  fn compass_letters_and_sector_counts() {
    assert_eq!(compass8(0.0), "N");
    assert_eq!(compass8(359.0), "N");
    assert_eq!(compass8(22.4), "N");
    assert_eq!(compass8(22.6), "NE");
    assert_eq!(compass8(52.2), "NE");
    assert_eq!(compass8(270.0), "W");
    assert_eq!(compass8(-90.0), "W");
    let c = sector_counts([0.0, 10.0, 350.0, 90.0, 52.0, f64::NAN]);
    assert_eq!(c, [3, 1, 1, 0, 0, 0, 0, 0], "NaN 不计入");
  }
}
