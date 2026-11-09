//! 等距圆柱投影（Plate Carrée）与折线简化工具。

/// 等距圆柱投影比例：经度每度 → `2` 像素。
pub(crate) const K: f64 = 2.0;
/// 世界画布宽/高（720×360 = 360°×2 × 180°×2）。
pub(crate) const MAP_W: f64 = 720.0;
pub(crate) const MAP_H: f64 = 360.0;

/// 世界内容在 x 方向的平铺偏移（×MAP_W），用于反子午线环绕：左/中/右三份。
pub(crate) const WORLD_OFFSETS: [f64; 3] = [-1.0, 0.0, 1.0];

/// 第 `i` 份平铺的建造次序（0 = 最先建）：离中间那份（偏移 `0`）越近越先建，距离相同时靠左的先。
///
/// 初始视口正好是偏移为 `0` 的那一份，其余几份要平移到反子午线附近才看得见（见 `map_view`）。
/// 按偏移的**数值**算而不是写死下标：以后改了 [`WORLD_OFFSETS`] 的顺序或份数，次序跟着变，
/// 不会悄悄变成「先建了看不见的那份」。
pub(crate) fn tile_build_rank(i: usize) -> usize {
  let key = |j: usize| (WORLD_OFFSETS[j].abs().to_bits(), j);
  let mine = key(i);
  (0..WORLD_OFFSETS.len()).filter(|&j| key(j) < mine).count()
}

/// 等距圆柱投影：经纬度 → SVG 坐标。
pub fn project(lon: f64, lat: f64) -> (f64, f64) {
  ((lon + 180.0) * K, (90.0 - lat) * K)
}

/// 多边形 → SVG points 字符串。
pub(crate) fn polygon_points(poly: &[(f64, f64)]) -> String {
  // 每个点约 10 个字符（`xxx.x,yyy.y `）：一次分配够用，不为每个点单独分配再拼。
  let mut out = String::with_capacity(poly.len() * 11);
  write_polygon_points(&mut out, poly);
  out
}

/// 同 [`polygon_points`]，直接写进调用方的缓冲（拼整段 SVG 标记时省掉中间字符串）。
pub(crate) fn write_polygon_points(out: &mut String, poly: &[(f64, f64)]) {
  use std::fmt::Write;
  for (i, &(lon, lat)) in poly.iter().enumerate() {
    if i > 0 {
      out.push(' ');
    }
    let (x, y) = project(lon, lat);
    // 写进 `String` 不会失败。
    let _ = write!(out, "{x:.1},{y:.1}");
  }
}

/// Douglas–Peucker 折线简化：删除与弦距离小于 `tol`（单位：度）的点。
pub(crate) fn simplify(poly: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
  if poly.len() < 3 {
    return poly.to_vec();
  }
  let mut keep = vec![false; poly.len()];
  keep[0] = true;
  keep[poly.len() - 1] = true;
  let tol2 = tol * tol;
  let mut stack = vec![(0usize, poly.len() - 1)];

  while let Some((a, b)) = stack.pop() {
    if b <= a + 1 {
      continue;
    }
    let (ax, ay) = poly[a];
    let (bx, by) = poly[b];
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    let (mut max_d, mut max_i) = (0.0f64, a);
    for (i, &(px, py)) in poly.iter().enumerate().take(b).skip(a + 1) {
      let t = if len2 == 0.0 {
        0.0
      } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
      };
      let (qx, qy) = (ax + t * dx, ay + t * dy);
      let d = (px - qx) * (px - qx) + (py - qy) * (py - qy);
      if d > max_d {
        max_d = d;
        max_i = i;
      }
    }
    if max_d > tol2 {
      keep[max_i] = true;
      stack.push((a, max_i));
      stack.push((max_i, b));
    }
  }

  poly
    .iter()
    .zip(keep.iter())
    .filter_map(|(p, &k)| k.then_some(*p))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tile_build_order_is_centre_first_then_left_then_right() {
    // 次序：偏移 0（中）→ -1（左）→ +1（右）；每个次序恰好出现一次。
    let ranks: Vec<usize> = (0..WORLD_OFFSETS.len()).map(tile_build_rank).collect();
    assert_eq!(ranks, [1, 0, 2]);
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..WORLD_OFFSETS.len()).collect::<Vec<_>>());
    // 最先建的必须是视口所在的那份（偏移为 0）。
    let first = ranks.iter().position(|&r| r == 0).expect("有第 0 份");
    assert_eq!(WORLD_OFFSETS[first], 0.0);
  }

  #[test]
  fn project_linear_mapping() {
    assert_eq!(project(-180.0, 90.0), (0.0, 0.0));
    assert_eq!(project(0.0, 0.0), (360.0, 180.0));
    assert_eq!(project(180.0, -90.0), (720.0, 360.0));
  }

  #[test]
  fn polygon_points_are_one_decimal_pairs_joined_by_spaces() {
    // 叠加层改成拼整段标记之后，points 的格式是 e2e 与样式都认的契约：不能变。
    assert_eq!(
      polygon_points(&[(-180.0, 90.0), (0.0, 0.0)]),
      "0.0,0.0 360.0,180.0"
    );
    assert_eq!(polygon_points(&[(10.04, 20.06)]), "380.1,139.9");
    assert_eq!(polygon_points(&[]), "");
    // 直接写进调用方的缓冲：接在已有内容后面，不清空、不多加分隔符。
    let mut buf = String::from("p=");
    write_polygon_points(&mut buf, &[(0.0, 0.0), (1.0, 1.0)]);
    assert_eq!(buf, "p=360.0,180.0 362.0,178.0");
  }

  #[test]
  fn simplify_collinear_keeps_endpoints() {
    let poly = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)];
    assert_eq!(simplify(&poly, 0.5), vec![(0.0, 0.0), (3.0, 0.0)]);
  }

  #[test]
  fn simplify_keeps_corner() {
    let poly = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    assert_eq!(simplify(&poly, 0.1).len(), 3);
  }
}
