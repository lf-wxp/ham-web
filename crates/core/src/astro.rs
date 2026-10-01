//! 太阳几何：赤纬、直射点经度与晨昏圈（灰线）计算。
//!
//! 纯函数、零依赖，供灰线地图与网格地图的「灰线叠加」共用，
//! 并可在原生目标下做单元测试。

/// 一年中的第几天（1–366）。
#[must_use]
pub fn day_of_year(year: u32, month: u32, day: u32) -> u32 {
  let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
  let cum = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
  let mut doy = cum[(month - 1) as usize] + day;
  if leap && month > 2 {
    doy += 1;
  }
  doy
}

/// 太阳赤纬（度，北半球为正），近似公式。
#[must_use]
pub fn solar_declination(day_of_year: u32) -> f64 {
  23.44 * (2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.25).sin()
}

/// 太阳直射点经度（度，东经为正），由 UTC 小时（0–24）推算。
#[must_use]
pub fn subsolar_longitude(utc_hour: f64) -> f64 {
  15.0 * (12.0 - utc_hour)
}

/// 某经度处的晨昏圈纬度（度）。赤纬接近 0 时做限幅避免除零。
#[must_use]
pub fn terminator_lat(lon: f64, decl: f64, subsolar_lon: f64) -> f64 {
  let d = if decl.abs() < 1.0 {
    1.0 * decl.signum()
  } else {
    decl
  };
  let h = (lon - subsolar_lon).to_radians();
  (-h.cos() / d.to_radians().tan()).atan().to_degrees()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn day_of_year_handles_leap_years() {
    assert_eq!(day_of_year(2024, 1, 1), 1);
    assert_eq!(day_of_year(2024, 3, 1), 61); // 闰年：31 + 29 + 1
    assert_eq!(day_of_year(2023, 3, 1), 60); // 平年：31 + 28 + 1
    assert_eq!(day_of_year(2024, 12, 31), 366);
    assert_eq!(day_of_year(2023, 12, 31), 365);
  }

  #[test]
  fn declination_extremes_and_equinox() {
    // 夏至前后（约第 172 天）赤纬最大（北），冬至（约第 355 天）最小（南）。
    assert!(solar_declination(172) > 23.0);
    assert!(solar_declination(355) < -23.0);
    // 春分（约第 81 天）赤纬接近 0。
    assert!(solar_declination(81).abs() < 0.1);
  }

  #[test]
  fn subsolar_longitude_aligns_with_utc() {
    assert!((subsolar_longitude(12.0) - 0.0).abs() < 1e-9); // 正午在本初子午线
    assert!((subsolar_longitude(0.0) - 180.0).abs() < 1e-9); // 午夜在反子午线
    assert!((subsolar_longitude(6.0) - 90.0).abs() < 1e-9); // 06 UTC 在 90°E
  }

  #[test]
  fn terminator_lat_at_meridians() {
    // 直射点经线上晨昏圈位于 −(90°−|decl|)，反日点经线上位于 +(90°−|decl|)。
    let decl = 20.0;
    assert!((terminator_lat(0.0, decl, 0.0) + 70.0).abs() < 1e-9);
    assert!((terminator_lat(180.0, decl, 0.0) - 70.0).abs() < 1e-9);
  }
}
