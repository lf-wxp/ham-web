use crate::pages::map::project;
use ham_web_core::grid::great_circle_path;

/// 大圆航线 → SVG path 数据。经度连续展开（可超出 ±180°），跨反子午线时由
/// MapView 的平铺副本补齐，不会出现横贯全图的折线。
pub(super) fn great_circle_d(from: (f64, f64), to: (f64, f64), n: usize) -> String {
  let mut prev: Option<f64> = None;
  great_circle_path(from.0, from.1, to.0, to.1, n)
    .into_iter()
    .map(|(la, mut lo)| {
      if let Some(p) = prev {
        while lo - p > 180.0 {
          lo -= 360.0;
        }
        while p - lo > 180.0 {
          lo += 360.0;
        }
      }
      prev = Some(lo);
      let (x, y) = project(lo, la);
      format!("{x:.1},{y:.1}")
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// 全局 square 索引 `(sl, sa)` → 4 位网格码字符串。
pub(super) fn square_label((sl, sa): (usize, usize)) -> String {
  let fl = sl / 10;
  let fa = sa / 10;
  let sq_lon = sl % 10;
  let sq_lat = sa % 10;
  format!(
    "{}{}{}{}",
    (b'A' + fl.min(17) as u8) as char,
    (b'A' + fa.min(17) as u8) as char,
    sq_lon,
    sq_lat,
  )
}

/// 全局 square 索引 → 中心经纬度 `(纬度, 经度)`。
pub(super) fn square_center((sl, sa): (usize, usize)) -> (f64, f64) {
  let lon = -180.0 + sl as f64 * 2.0 + 1.0;
  let lat = 90.0 - sa as f64 - 0.5;
  (lat, lon)
}
