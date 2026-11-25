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

/// 把闭区间上边界拉回最后一格内部的微小量（远小于最小格 `1/24°`）。
const UPPER_EDGE_EPS: f64 = 1e-9;

/// 经纬度转 6 位 Maidenhead 网格（如 `PM45AA`）。经纬度越界返回 `None`。
#[must_use]
pub fn grid_from_lat_lon(lat: f64, lon: f64) -> Option<String> {
  if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
    return None;
  }
  // 闭区间的上边界（`lat=90` / `lon=180`）归入最后一格：不夹紧的话 field 索引是 18（被
  // `field_char` 截成 `R`），而 square / sub 用的是取模结果 0，拼出落在约 80°N 的错误网格。
  let lon = (lon + 180.0).min(360.0 - UPPER_EDGE_EPS);
  let lat = (lat + 90.0).min(180.0 - UPPER_EDGE_EPS);

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

/// 大圆航线采样：在两点间沿大圆均匀插值 `n` 段，返回 `(纬度, 经度)` 序列（含两端点）。
///
/// 用于地图绘制两点间最短路径；对跖点（几乎正对）时大圆退化，退化为绕一个垂直轴
/// 的任意半圆（仍是一条合法的大圆）。
#[must_use]
pub fn great_circle_path(lat1: f64, lon1: f64, lat2: f64, lon2: f64, n: usize) -> Vec<(f64, f64)> {
  let to_vec = |lat: f64, lon: f64| -> (f64, f64, f64) {
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    (la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin())
  };
  let to_latlon = |(x, y, z): (f64, f64, f64)| -> (f64, f64) {
    (
      z.atan2((x * x + y * y).sqrt()).to_degrees(),
      y.atan2(x).to_degrees(),
    )
  };

  let v1 = to_vec(lat1, lon1);
  let v2 = to_vec(lat2, lon2);
  let dot = (v1.0 * v2.0 + v1.1 * v2.1 + v1.2 * v2.2).clamp(-1.0, 1.0);

  let mut out = Vec::with_capacity(n + 1);
  if dot < -0.999_999_9 {
    // 对跖点：选一个不平行于 v1 的参考向量做叉积，绕该垂直轴旋转半圆。
    let axis = if v1.0.abs() < 0.9 {
      (1.0, 0.0, 0.0)
    } else {
      (0.0, 1.0, 0.0)
    };
    let (mut cx, mut cy, mut cz) = (
      v1.1 * axis.2 - v1.2 * axis.1,
      v1.2 * axis.0 - v1.0 * axis.2,
      v1.0 * axis.1 - v1.1 * axis.0,
    );
    let len = (cx * cx + cy * cy + cz * cz).sqrt();
    (cx, cy, cz) = (cx / len, cy / len, cz / len);
    for i in 0..=n {
      let t = i as f64 / n as f64;
      let ang = t * std::f64::consts::PI;
      let (x, y, z) = (
        v1.0 * ang.cos() + cx * ang.sin(),
        v1.1 * ang.cos() + cy * ang.sin(),
        v1.2 * ang.cos() + cz * ang.sin(),
      );
      out.push(to_latlon((x, y, z)));
    }
    return out;
  }

  let omega = dot.acos();
  let sin_omega = omega.sin();
  for i in 0..=n {
    let t = i as f64 / n as f64;
    let a = ((1.0 - t) * omega).sin() / sin_omega;
    let b = (t * omega).sin() / sin_omega;
    let (x, y, z) = (
      a * v1.0 + b * v2.0,
      a * v1.1 + b * v2.1,
      a * v1.2 + b * v2.2,
    );
    out.push(to_latlon((x, y, z)));
  }
  out
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

/// 球面上以 `(lat, lon)` 为心、`radius_km` 为半径的圆：沿方位角 0°、360°/n、… 取 `n` 个点（不重复首点），
/// 返回 `(纬度, 经度)`。半径与 [`distance_bearing`] 用同一个地球半径（6371 km）。
///
/// **经度不归一化**：圆跨过日期变更线时经度会超出 ±180°（如 `180.3`），这样相邻点之间始终连续，
/// 画成折线不会拦腰拉出一条横穿地图的线；地图把世界在横向平铺了三份，越界的那一段自然落在邻图上。
///
/// 输入不合法（中心越界、半径不是正的有限值、`n < 3`、半径超过半个地球周长）返回空。
/// 圆盖住极点时经度不再单调，本函数不处理（无线电地平线只有几十到几百公里）。
#[must_use]
pub fn circle_points(lat: f64, lon: f64, radius_km: f64, n: usize) -> Vec<(f64, f64)> {
  const R: f64 = 6371.0;
  if !(lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat)) {
    return Vec::new();
  }
  if !(radius_km.is_finite() && radius_km > 0.0 && radius_km < std::f64::consts::PI * R) || n < 3 {
    return Vec::new();
  }
  let delta = radius_km / R;
  let (phi1, lam1) = (lat.to_radians(), lon.to_radians());
  (0..n)
    .map(|i| {
      let theta = std::f64::consts::TAU * i as f64 / n as f64;
      let phi2 = (phi1.sin() * delta.cos() + phi1.cos() * delta.sin() * theta.cos())
        .clamp(-1.0, 1.0)
        .asin();
      let lam2 = lam1
        + (theta.sin() * delta.sin() * phi1.cos()).atan2(delta.cos() - phi1.sin() * phi2.sin());
      (phi2.to_degrees(), lam2.to_degrees())
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn upper_edges_fall_into_the_last_cell() {
    // 北极 / 日期变更线东侧：此前 lat=90 得到约 80°N 的 `JR..` 错格。
    assert_eq!(grid_from_lat_lon(90.0, 0.0).as_deref(), Some("JR09AX"));
    assert_eq!(grid_from_lat_lon(0.0, 180.0).as_deref(), Some("RJ90XA"));
    assert_eq!(grid_from_lat_lon(90.0, 180.0).as_deref(), Some("RR99XX"));
    // 下边界不变。
    assert_eq!(grid_from_lat_lon(-90.0, -180.0).as_deref(), Some("AA00AA"));
    // 往返：最后一格的中心仍在边界内。
    let (lat, lon) = lat_lon_from_grid("RR99XX").expect("valid");
    assert!(
      lat > 89.9 && lat < 90.0 && lon > 179.9 && lon < 180.0,
      "{lat},{lon}"
    );
  }

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

  #[test]
  fn great_circle_path_equator_quarter() {
    // 赤道 (0,0) → (0,90)：非对跖，中间点经度 45、纬度 0。
    let pts = great_circle_path(0.0, 0.0, 0.0, 90.0, 100);
    assert_eq!(pts.first(), Some(&(0.0, 0.0)));
    assert_eq!(pts.last(), Some(&(0.0, 90.0)));
    let mid = pts[50];
    assert!((mid.1 - 45.0).abs() < 1e-6, "lon {}", mid.1);
    assert!(mid.0.abs() < 1e-6, "lat {}", mid.0);
  }

  #[test]
  fn great_circle_path_antipodal_endpoints() {
    // 对跖点：仍返回含两端点的路径（经度 ±180 等价）。
    let pts = great_circle_path(0.0, 0.0, 0.0, 180.0, 50);
    assert_eq!(pts.len(), 51);
    assert_eq!(pts[0], (0.0, 0.0));
    assert!((pts[50].0 - 0.0).abs() < 1e-6);
    assert!((pts[50].1 - 180.0).abs() < 1e-6);
  }

  #[test]
  fn circle_points_are_all_at_the_radius_and_evenly_spread() {
    // 各取一个北、南半球，低、中、高纬与跨经度的中心；半径从市区到省域。
    for (lat, lon) in [
      (31.2, 121.5),
      (-33.9, 151.2),
      (0.0, 0.0),
      (64.0, -150.0),
      (-80.0, 20.0),
    ] {
      for radius in [25.0, 300.0] {
        let pts = circle_points(lat, lon, radius, 72);
        assert_eq!(pts.len(), 72);
        for (i, &(plat, plon)) in pts.iter().enumerate() {
          let (d, b) = distance_bearing(lat, lon, plat, plon);
          assert!(
            (d - radius).abs() < 1e-6,
            "({lat},{lon}) r={radius} #{i}: d={d}"
          );
          // 方位角按 5° 一步均匀排开，第 0 个点正北；比较时处理 0° / 360° 的回绕。
          let expect = 5.0 * i as f64;
          let diff = ((b - expect + 540.0) % 360.0) - 180.0;
          assert!(
            diff.abs() < 1e-6,
            "({lat},{lon}) #{i}: 方位 {b}，应为 {expect}"
          );
        }
      }
    }
  }

  #[test]
  fn circle_north_point_is_one_degree_of_latitude_per_111_km() {
    // 手算：地球半径 6371 km 时纬度 1° = 6371·π/180 km；正北方向的点只改纬度、不改经度。
    let one_degree_km = 6371.0 * std::f64::consts::PI / 180.0;
    let pts = circle_points(31.2, 121.5, one_degree_km, 8);
    assert!((pts[0].0 - 32.2).abs() < 1e-9, "{:?}", pts[0]);
    assert!((pts[0].1 - 121.5).abs() < 1e-9, "{:?}", pts[0]);
    // 正南方向（第 4 个）同理回到 30.2°。
    assert!((pts[4].0 - 30.2).abs() < 1e-9, "{:?}", pts[4]);
  }

  #[test]
  fn circle_across_the_antimeridian_stays_continuous() {
    let pts = circle_points(10.0, 179.9, 50.0, 90);
    let max_lon = pts.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    assert!(
      max_lon > 180.0,
      "越过日期变更线的一侧应是 >180 的连续经度，实际最大 {max_lon}"
    );
    for w in pts
      .windows(2)
      .chain(std::iter::once(&[pts[pts.len() - 1], pts[0]][..]))
    {
      assert!((w[1].1 - w[0].1).abs() < 1.0, "相邻点经度不该跳变：{:?}", w);
    }
  }

  #[test]
  fn circle_rejects_what_it_cannot_draw() {
    assert!(
      circle_points(31.0, 121.0, 0.0, 36).is_empty(),
      "半径 0 没有圆"
    );
    assert!(circle_points(31.0, 121.0, -5.0, 36).is_empty());
    assert!(circle_points(31.0, 121.0, f64::NAN, 36).is_empty());
    assert!(circle_points(31.0, 121.0, f64::INFINITY, 36).is_empty());
    assert!(
      circle_points(31.0, 121.0, 20_100.0, 36).is_empty(),
      "超过半个地球周长"
    );
    assert!(circle_points(91.0, 121.0, 10.0, 36).is_empty());
    assert!(circle_points(f64::NAN, 121.0, 10.0, 36).is_empty());
    assert!(circle_points(31.0, 121.0, 10.0, 2).is_empty());
  }
}
