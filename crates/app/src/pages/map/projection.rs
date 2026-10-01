//! 等距圆柱投影（Plate Carrée）与折线简化工具。

/// 等距圆柱投影比例：经度每度 → `2` 像素。
pub(crate) const K: f64 = 2.0;
/// 世界画布宽/高（720×360 = 360°×2 × 180°×2）。
pub(crate) const MAP_W: f64 = 720.0;
pub(crate) const MAP_H: f64 = 360.0;

/// 世界内容在 x 方向的平铺偏移（×MAP_W），用于反子午线环绕：左/中/右三份。
pub(crate) const WORLD_OFFSETS: [f64; 3] = [-1.0, 0.0, 1.0];

/// 等距圆柱投影：经纬度 → SVG 坐标。
pub fn project(lon: f64, lat: f64) -> (f64, f64) {
  ((lon + 180.0) * K, (90.0 - lat) * K)
}

/// 多边形 → SVG points 字符串。
pub(crate) fn polygon_points(poly: &[(f64, f64)]) -> String {
  poly
    .iter()
    .map(|&(lon, lat)| {
      let (x, y) = project(lon, lat);
      format!("{x:.1},{y:.1}")
    })
    .collect::<Vec<_>>()
    .join(" ")
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
  fn project_linear_mapping() {
    assert_eq!(project(-180.0, 90.0), (0.0, 0.0));
    assert_eq!(project(0.0, 0.0), (360.0, 180.0));
    assert_eq!(project(180.0, -90.0), (720.0, 360.0));
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
