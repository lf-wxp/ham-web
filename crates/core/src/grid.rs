//! Maidenhead 网格定位（Grid Locator）：经纬度 ↔ 网格编码换算。

/// 把 field 字母转索引：`A`→0 … `R`→17（前两位字段，共 18 个）。
fn field_value(c: char) -> Option<usize> {
  let u = c.to_ascii_uppercase();
  if ('A'..='R').contains(&u) {
    Some(u as usize - 'A' as usize)
  } else {
    None
  }
}

/// 把 subsquare 字母转索引：`A`→0 … `X`→23（第 5、6 位子方格，共 24 个）。
fn subsquare_value(c: char) -> Option<usize> {
  let u = c.to_ascii_uppercase();
  if ('A'..='X').contains(&u) {
    Some(u as usize - 'A' as usize)
  } else {
    None
  }
}

/// field 索引 → 字母；越界（如 `lat=90`/`lon=180` 得到的 18）截断到 `R`。
fn field_char(v: usize) -> char {
  (b'A' + v.min(17) as u8) as char
}

/// subsquare 索引 → 字母；越界截断到 `X`。
fn subsquare_char(v: usize) -> char {
  (b'A' + v.min(23) as u8) as char
}

/// 经纬度转 6 位 Maidenhead 网格（如 `PM45AA`）。经纬度越界返回 `None`。
#[must_use]
pub fn grid_from_lat_lon(lat: f64, lon: f64) -> Option<String> {
  if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
    return None;
  }
  let lon = lon + 180.0;
  let lat = lat + 90.0;

  let field_lon = (lon / 20.0).floor() as usize;
  let field_lat = (lat / 10.0).floor() as usize;
  let square_lon = ((lon % 20.0) / 2.0).floor() as usize;
  let square_lat = ((lat % 10.0) / 1.0).floor() as usize;
  let sub_lon = ((lon % 2.0) / (2.0 / 24.0)).floor() as usize;
  let sub_lat = ((lat % 1.0) / (1.0 / 24.0)).floor() as usize;

  Some(format!(
    "{}{}{}{}{}{}",
    field_char(field_lon),
    field_char(field_lat),
    square_lon,
    square_lat,
    subsquare_char(sub_lon),
    subsquare_char(sub_lat),
  ))
}

/// Maidenhead 网格转经纬度，返回网格中心点 `(纬度, 经度)`。至少 4 位。
#[must_use]
pub fn lat_lon_from_grid(grid: &str) -> Option<(f64, f64)> {
  let g = grid.trim().to_ascii_uppercase();
  let chars: Vec<char> = g.chars().collect();
  if chars.len() != 4 && chars.len() != 6 {
    return None;
  }

  let lon_field = field_value(chars[0])? as f64 * 20.0;
  let lat_field = field_value(chars[1])? as f64 * 10.0;
  let lon_square = chars[2].to_digit(10)? as f64 * 2.0;
  let lat_square = chars[3].to_digit(10)? as f64 * 1.0;

  let (lon, lat, w, h) = if chars.len() == 6 {
    let lon_sub = subsquare_value(chars[4])? as f64 * (2.0 / 24.0);
    let lat_sub = subsquare_value(chars[5])? as f64 * (1.0 / 24.0);
    (
      lon_field + lon_square + lon_sub,
      lat_field + lat_square + lat_sub,
      2.0 / 24.0,
      1.0 / 24.0,
    )
  } else {
    (lon_field + lon_square, lat_field + lat_square, 2.0, 1.0)
  };

  Some((lat - 90.0 + h / 2.0, lon - 180.0 + w / 2.0))
}

/// 网格码转 field 索引 `(经度 field, 纬度 field)`，`A`=0 … `R`=17（前 2 字符）。
#[must_use]
pub fn field_index(grid: &str) -> Option<(usize, usize)> {
  let chars: Vec<char> = grid.trim().to_ascii_uppercase().chars().collect();
  if chars.len() < 2 {
    return None;
  }
  Some((field_value(chars[0])?, field_value(chars[1])?))
}

/// 网格码转全局 square 索引 `(经度 square, 纬度 square)`，各 0–179（前 4 字符，2°×1°）。
#[must_use]
pub fn square_index(grid: &str) -> Option<(usize, usize)> {
  let chars: Vec<char> = grid.trim().to_ascii_uppercase().chars().collect();
  if chars.len() < 4 {
    return None;
  }
  let fl = field_value(chars[0])?;
  let fa = field_value(chars[1])?;
  let sl = chars[2].to_digit(10)? as usize;
  let sa = chars[3].to_digit(10)? as usize;
  Some((fl * 10 + sl, fa * 10 + sa))
}

/// 两点大圆距离（km）与初始方位角（度，正北为 0，顺时针）。
/// 输入为 `(纬度, 经度)`，单位度。
#[must_use]
pub fn distance_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
  let (lat1, lon1, lat2, lon2) = (
    lat1.to_radians(),
    lon1.to_radians(),
    lat2.to_radians(),
    lon2.to_radians(),
  );
  let dlat = lat2 - lat1;
  let dlon = lon2 - lon1;
  let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
  let a = a.clamp(0.0, 1.0);
  let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
  let dist = 6371.0 * c;

  let y = dlon.sin() * lat2.cos();
  let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
  let bearing = (y.atan2(x).to_degrees() + 360.0) % 360.0;

  (dist, bearing)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zero_is_jj00aa() {
    assert_eq!(grid_from_lat_lon(0.0, 0.0).as_deref(), Some("JJ00AA"));
  }

  #[test]
  fn roundtrip_center() {
    // 4 位网格 "JJ00" 覆盖 2°×1°，中心为 (0.5, 1.0)
    let (lat, lon) = lat_lon_from_grid("JJ00").expect("valid");
    assert!((lat - 0.5).abs() < 1e-6, "lat {lat}");
    assert!((lon - 1.0).abs() < 1e-6, "lon {lon}");
  }

  #[test]
  fn invalid_inputs() {
    assert_eq!(grid_from_lat_lon(91.0, 0.0), None);
    assert_eq!(lat_lon_from_grid("XX"), None);
  }

  #[test]
  fn four_digit_grid() {
    let g = grid_from_lat_lon(39.9, 116.4).expect("valid");
    assert_eq!(g.len(), 6);
    // 前四位可逆回大致中心
    let (lat, lon) = lat_lon_from_grid(&g[..4]).expect("valid");
    assert!((lat - 39.9).abs() < 1.0);
    assert!((lon - 116.4).abs() < 2.0);
  }

  #[test]
  fn field_index_parses_first_two_chars() {
    assert_eq!(field_index("OM89EW"), Some((14, 12)));
    assert_eq!(field_index("om89ew"), Some((14, 12)));
    assert_eq!(field_index("JJ00"), Some((9, 9)));
    assert_eq!(field_index("X"), None);
  }

  #[test]
  fn square_index_parses_first_four_chars() {
    assert_eq!(square_index("OM89EW"), Some((14 * 10 + 8, 12 * 10 + 9)));
    assert_eq!(square_index("JJ00"), Some((9 * 10, 9 * 10)));
    assert_eq!(square_index("XX"), None);
  }

  #[test]
  fn distance_bearing_same_point_zero() {
    let (d, _) = distance_bearing(30.0, 120.0, 30.0, 120.0);
    assert!(d < 1e-6);
  }

  #[test]
  fn distance_bearing_beijing_to_london() {
    // 北京（39.9N, 116.4E）→ 伦敦（51.5N, 0.1W）约 8150 km，朝西北（约 320°）。
    let (d, b) = distance_bearing(39.9, 116.4, 51.5, -0.1);
    assert!((d - 8150.0).abs() < 250.0, "dist {d}");
    assert!(b > 300.0 && b < 335.0, "bearing {b}");
  }
}
