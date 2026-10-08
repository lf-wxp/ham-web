//! 三维方向图球面采样 → GPU 三角网格（纯数学，不依赖浏览器）。
//!
//! [`crate::nec::NecResult::pattern3d`] 已经是「θ 自天顶量起、φ 自 0 起」的规则采样，
//! 这里把它折成可直接上传 WebGL 的三角网格：顶点取「单位方向 × 幅度」，幅度
//! `= 10^{(dB − 峰值)/20}`（与 SVG 线框同一口径，球壳的凹凸就是方向图的真实形状），
//! 低于峰值 [`MESH_DB_FLOOR`] 的分量夹到下限 —— 否则单个零点会把整张曲面压成一点。
//!
//! 顶点法线取参数切线的叉积 `cross(∂P/∂θ, ∂P/∂φ)`（已知朝外），而不是三角形面法线
//! 累加：极点上「一圈顶点位置重合」会让面法线贡献不对称，法线被拉偏、Gouraud 插值下
//! 出现风车状伪影；参数切线天然对称。极点处 `∂P/∂φ = 0`、叉积退化，此时回退到径向
//! 方向 —— 对球面参数化那正是极点的真实法线，因此不会出现 NaN。
//!
//! 三角顶点顺序保证**逆时针朝外**，于是 `cross(e1, e2)` 直接给出朝外的面法线；
//! 顶点位置可能很小（被夹到下限的零点），但方向信息由参数网格给出，不受影响。
//!
//! 之所以放在核心 crate：它是纯几何变换，与浏览器无关，可以脱离 WASM 单测
//! （见文件末尾），而渲染端只负责上传缓冲区。

use std::f64::consts::PI;

use crate::nec::NecResult;

/// 方向图低于峰值多少 dB 就夹到下限（与页面 SVG 线框一致）。
pub const MESH_DB_FLOOR: f64 = -40.0;

/// 参考球线框的规模上限：顶点数 ≈ `(rings + meridians) × samples × 2`，
/// 无上限时一个笔误就能构造出百万级顶点、把 GPU 上传卡住。
const MAX_WIREFRAME_LINES: usize = 128;
/// 单条线框折线的最大段数（同上）。
const MAX_WIREFRAME_SAMPLES: usize = 512;

/// 方向图网格的顶点数上限（`(nt + 1) × np`）。
///
/// 这个函数是 `pub` 的，而顶点数、`with_capacity` 的分配量与三角形索引全由 `shape`
/// 决定：没有上限时，上游把形状传反（例如 `(90, 361)`）就能构造出几万顶点、几十万索引；
/// 更大时 `vid` 里的 `as u32` 还会**静默回绕**，把三角形指到别的顶点上 —— 表现为
/// 「画出来一团乱」，而不是报错。当前调用方传 `(18, 36)`，余量足够。
const MAX_MESH_VERTICES: usize = 1 << 16;

/// 可直接上传的三角网格。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PatternMesh {
  /// 顶点位置：单位球方向 × 幅度。
  pub positions: Vec<[f32; 3]>,
  /// 顶点法线（单位向量，指向球外）。
  pub normals: Vec<[f32; 3]>,
  /// 三角形索引（每三个一组）。
  pub indices: Vec<u32>,
}

impl PatternMesh {
  /// 顶点数。
  #[must_use]
  pub fn vertex_count(&self) -> usize {
    self.positions.len()
  }

  /// 三角形数。
  #[must_use]
  pub fn triangle_count(&self) -> usize {
    self.indices.len() / 3
  }

  /// 是否没有任何可绘制的三角形。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.indices.is_empty() || self.positions.is_empty()
  }
}

/// 由 `NecResult` 里的方向图网格构建三角网格。
#[must_use]
pub fn build_pattern_mesh(r: &NecResult) -> PatternMesh {
  mesh_from_grid(r.pattern3d_shape, &r.pattern3d, r.gain_max_dbi)
}

/// 由「`(θ 行数, φ 列数)` + 平坦的 dB 数组 + 峰值」构建三角网格。
///
/// 网格约定与 [`crate::nec::NecResult::pattern3d`] 一致：`θ` 取 `0..=180°` 共
/// `nt + 1` 行，`φ` 取 `0..360°`（不含终点）共 `np` 列，数据按行优先排列。
/// `shape` 非法（行 / 列不足或数组长度不够）时返回空网格，而不是 panic ——
/// 调用方拿到空网格就会画不出东西，不会崩掉页面。
#[must_use]
pub fn mesh_from_grid(shape: (usize, usize), values: &[f64], peak_dbi: f64) -> PatternMesh {
  let (nt, np) = shape;
  // `(nt + 1) * np` 既可能溢出 `usize`（debug 下直接 panic），也必须限规模（见
  // [`MAX_MESH_VERTICES`]）。先在 `checked_mul` 上算一遍，下面的判断与分配都用它。
  let Some(vertices) = nt.checked_add(1).and_then(|rows| rows.checked_mul(np)) else {
    return PatternMesh::default();
  };
  if nt < 1 || np < 3 || vertices > MAX_MESH_VERTICES || values.len() < vertices {
    return PatternMesh::default();
  }
  // 峰值非有限时 `amplitude` 里的 `(db - NaN).max(下限)` 会恒定取下限，整张网格被压成
  // 一个半径 0.01 的小球 —— 看着像画出来了，其实是错的。直接给空网格更好发现。
  if !peak_dbi.is_finite() {
    return PatternMesh::default();
  }
  let mut positions = Vec::with_capacity(vertices);
  for i in 0..=nt {
    let (sin_t, cos_t) = (PI * i as f64 / nt as f64).sin_cos();
    for j in 0..np {
      let (sin_p, cos_p) = (2.0 * PI * j as f64 / np as f64).sin_cos();
      let amp = amplitude(values[i * np + j], peak_dbi);
      positions.push([
        (amp * sin_t * cos_p) as f32,
        (amp * sin_t * sin_p) as f32,
        (amp * cos_t) as f32,
      ]);
    }
  }
  let vid = |i: usize, j: usize| -> u32 { (i * np + (j % np)) as u32 };
  let mut indices = Vec::with_capacity(nt * np * 6);
  for i in 0..nt {
    for j in 0..np {
      // 四边形 (a, b, c, d)：`a=(i,j)`、`b=(i,j+1)`、`c=(i+1,j+1)`、`d=(i+1,j)`。
      // 两个三角形都取「先 θ 后 φ」的顺序，`cross(e1, e2)` 才朝外。
      let a = vid(i, j);
      let b = vid(i, j + 1);
      let c = vid(i + 1, j + 1);
      let d = vid(i + 1, j);
      indices.extend_from_slice(&[a, d, b, b, d, c]);
    }
  }
  let normals = vertex_normals(&positions, shape);
  PatternMesh {
    positions,
    normals,
    indices,
  }
}

/// 单位球参考线框，返回 `gl.LINES` 的**顶点对**列表。
///
/// `rings` 条纬线分布在 `(0, π)` 开区间（避免与极点退化），`meridians` 条经线均分
/// 一周，每条线用 `samples` 段折线逼近。参考球给出「0 dBi 的球」这一参照：
/// 方向图凹下去的地方就会露出它。
#[must_use]
pub fn unit_sphere_wireframe(rings: usize, meridians: usize, samples: usize) -> Vec<[f32; 3]> {
  // 上界是必要的：顶点数是 `(rings + meridians) × samples` 量级，构造出百万级顶点
  // 会让 GPU 上传卡住（当前调用方都传常量，这里是防将来传错）。
  let rings = rings.min(MAX_WIREFRAME_LINES);
  let meridians = meridians.min(MAX_WIREFRAME_LINES);
  let samples = samples.clamp(3, MAX_WIREFRAME_SAMPLES);
  let mut out = Vec::new();
  // 纬线：θ 在 (0, π) 开区间均分。
  for k in 1..=rings {
    let theta = PI * k as f64 / (rings + 1) as f64;
    let (sin_t, cos_t) = theta.sin_cos();
    for s in 0..samples {
      let (a, b) = (
        2.0 * PI * s as f64 / samples as f64,
        2.0 * PI * (s + 1) as f64 / samples as f64,
      );
      out.push([
        (sin_t * a.cos()) as f32,
        (sin_t * a.sin()) as f32,
        cos_t as f32,
      ]);
      out.push([
        (sin_t * b.cos()) as f32,
        (sin_t * b.sin()) as f32,
        cos_t as f32,
      ]);
    }
  }
  // 经线：φ 均分一周，θ 从 0 扫到 π。
  let meridian_samples = samples.max(4);
  for k in 0..meridians {
    let phi = 2.0 * PI * k as f64 / meridians as f64;
    let (sin_p, cos_p) = phi.sin_cos();
    for s in 0..meridian_samples {
      let (t0, t1) = (
        PI * s as f64 / meridian_samples as f64,
        PI * (s + 1) as f64 / meridian_samples as f64,
      );
      for t in [t0, t1] {
        let (sin_t, cos_t) = t.sin_cos();
        out.push([(sin_t * cos_p) as f32, (sin_t * sin_p) as f32, cos_t as f32]);
      }
    }
  }
  out
}

/// dB（相对峰值）→ 幅度，低于下限就夹住。
fn amplitude(db: f64, peak_dbi: f64) -> f64 {
  10f64.powf((db - peak_dbi).max(MESH_DB_FLOOR) / 20.0)
}

/// 用**参数切线的叉积**求顶点法线（`cross(∂P/∂θ, ∂P/∂φ)`，已知朝外）。
///
/// 不用三角形面法线累加：极点上「一圈顶点位置重合」会让相邻三角形的贡献不对称
/// （每个极点顶点只剩一个非退化三角形），法线被拉偏约半个 θ 步长，Gouraud 插值
/// 下会在极点出现风车状伪影。参数切线天然对称；极点处 `∂P/∂φ = 0`，
/// 叉积退化，此时回退到径向方向 —— 对球面参数化那正是极点的真实法线。
fn vertex_normals(positions: &[[f32; 3]], shape: (usize, usize)) -> Vec<[f32; 3]> {
  let (nt, np) = shape;
  let p = |i: usize, j: usize| -> [f64; 3] {
    let v = positions[i * np + (j % np)];
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
  };
  let mut out = Vec::with_capacity(positions.len());
  for i in 0..=nt {
    let i0 = i.saturating_sub(1);
    let i1 = (i + 1).min(nt);
    for j in 0..np {
      // 内层做中心差分，端点上退化为单侧差分（极点附近本来就只该取单侧）。
      let t_theta = sub(p(i1, j), p(i0, j));
      let t_phi = sub(p(i, j + 1), p(i, j + np - 1));
      let len_t = dot(t_theta, t_theta).sqrt();
      let len_p = dot(t_phi, t_phi).sqrt();
      // 极点处 `∂P/∂φ` 只在浮点噪声量级（sin 180° 不是精确的 0），不能靠绝对值
      // 判退化 —— 用「φ 切线相对 θ 切线可忽略」这个相对判据，否则会把噪声方向当成法线。
      let n = cross(t_theta, t_phi);
      let len_n = dot(n, n).sqrt();
      let v = if len_t > 1e-12 && len_p > 1e-6 * len_t && len_n > 0.0 {
        [n[0] / len_n, n[1] / len_n, n[2] / len_n]
      } else {
        radial(positions[i * np + j])
      };
      out.push([v[0] as f32, v[1] as f32, v[2] as f32]);
    }
  }
  out
}

/// 位置方向的单位向量；位置本身接近原点时退回 `+Z`。
fn radial(p: [f32; 3]) -> [f64; 3] {
  let v = [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])];
  let len = dot(v, v).sqrt();
  if len > 1e-12 {
    [v[0] / len, v[1] / len, v[2] / len]
  } else {
    [0.0, 0.0, 1.0]
  }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
  [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
  [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
  a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 取向量模长。
  fn len(v: [f32; 3]) -> f64 {
    (f64::from(v[0]).powi(2) + f64::from(v[1]).powi(2) + f64::from(v[2]).powi(2)).sqrt()
  }

  fn radius(db: f64, peak: f64) -> f64 {
    amplitude(db, peak)
  }

  #[test]
  fn grid_shape_and_indices_are_consistent() {
    let (nt, np) = (4usize, 8usize);
    let values = vec![0.0; (nt + 1) * np];
    let m = mesh_from_grid((nt, np), &values, 0.0);
    assert_eq!(m.vertex_count(), (nt + 1) * np);
    assert_eq!(m.normals.len(), m.positions.len());
    assert_eq!(m.triangle_count(), nt * np * 2);
    assert!(!m.is_empty());
    assert!(
      m.indices.iter().all(|&k| (k as usize) < m.positions.len()),
      "索引越界"
    );
  }

  #[test]
  fn malformed_grid_degrades_to_empty_mesh() {
    // 数组长度不够 / 行列太少，都不能 panic。
    assert!(mesh_from_grid((4, 8), &[0.0; 10], 0.0).is_empty());
    assert!(mesh_from_grid((0, 8), &[0.0; 8], 0.0).is_empty());
    assert!(mesh_from_grid((4, 2), &[0.0; 12], 0.0).is_empty());
    // 规模超上限（`MAX_MESH_VERTICES`）：返回空网格而不是巨量分配 ——
    // 否则 `vid` 里的 `as u32` 还会静默回绕，把三角形指到别的顶点上。
    let huge = MAX_MESH_VERTICES;
    assert!(mesh_from_grid((huge, 3), &vec![0.0; (huge + 1) * 3], 0.0).is_empty());
    // 极端值不能因为 `(nt + 1) * np` 溢出而在 debug 下 panic。
    assert!(mesh_from_grid((usize::MAX, 3), &[0.0; 4], 0.0).is_empty());
    assert!(mesh_from_grid((3, usize::MAX), &[0.0; 4], 0.0).is_empty());
  }

  #[test]
  fn uniform_sphere_has_unit_radii_and_outward_normals() {
    // 全 0 dBi（峰值也是 0）：每个顶点都在单位球上，法线朝外。
    let (nt, np) = (6usize, 12usize);
    let values = vec![0.0; (nt + 1) * np];
    let m = mesh_from_grid((nt, np), &values, 0.0);
    for (k, p) in m.positions.iter().enumerate() {
      assert!((len(*p) - 1.0).abs() < 1e-5, "顶点 {k} 不在单位球上：{p:?}");
      let n = m.normals[k];
      assert!((len(n) - 1.0).abs() < 1e-4, "法线 {k} 非单位向量：{n:?}");
      // 朝外：法线与径向同向（点积为正且接近 1）。
      let cos = (f64::from(p[0]) * f64::from(n[0])
        + f64::from(p[1]) * f64::from(n[1])
        + f64::from(p[2]) * f64::from(n[2]))
        / len(*p);
      assert!(cos > 0.99, "顶点 {k} 法线朝内：cos = {cos}");
    }
  }

  #[test]
  fn amplitude_is_clamped_at_the_floor() {
    // 0 dB → 1.0；−20 dB → 0.1；−80 dB 被夹到下限 −40 dB → 0.01。
    assert!((radius(0.0, 0.0) - 1.0).abs() < 1e-12);
    assert!((radius(-20.0, 0.0) - 0.1).abs() < 1e-12);
    assert!((radius(-80.0, 0.0) - 0.01).abs() < 1e-12);
    // 相对峰值：峰值 6 dBi、该点 −34 dBi 即相对 −40 dB。
    assert!((radius(-34.0, 6.0) - 0.01).abs() < 1e-12);
    assert!((radius(6.0, 6.0) - 1.0).abs() < 1e-12);
  }

  #[test]
  fn grid_radii_follow_the_gain_values() {
    // 只有 θ 变化、φ 不变：每一「行」的所有顶点半径相同。
    let (nt, np) = (4usize, 8usize);
    let mut values = vec![0.0; (nt + 1) * np];
    for i in 0..=nt {
      let db = -(i as f64) * 10.0;
      for j in 0..np {
        values[i * np + j] = db;
      }
    }
    let m = mesh_from_grid((nt, np), &values, 0.0);
    for i in 0..=nt {
      let expected = radius(-(i as f64) * 10.0, 0.0);
      for j in 0..np {
        let r = len(m.positions[i * np + j]);
        assert!(
          (r - expected).abs() < 1e-5,
          "第 {i} 行第 {j} 列半径 {r} ≠ {expected}"
        );
      }
    }
  }

  #[test]
  fn grounded_lower_half_collapses_to_the_floor() {
    // 有地面时下半空间填 −300 dB：超过下限，一律夹到 0.01。
    let (nt, np) = (4usize, 8usize);
    let mut values = vec![5.0; (nt + 1) * np];
    for i in (nt / 2 + 1)..=nt {
      for j in 0..np {
        values[i * np + j] = -300.0;
      }
    }
    let m = mesh_from_grid((nt, np), &values, 5.0);
    for i in (nt / 2 + 1)..=nt {
      for j in 0..np {
        let r = len(m.positions[i * np + j]);
        assert!((r - 0.01).abs() < 1e-6, "下半空间半径 {r} 未被夹住");
      }
    }
  }

  #[test]
  fn normals_are_finite_everywhere() {
    let (nt, np) = (18usize, 36usize);
    let mut values = Vec::with_capacity((nt + 1) * np);
    for i in 0..=nt {
      for _ in 0..np {
        // 造一个带深零点的方向图：θ = 0 与 θ = 180 都是零点。
        let t = PI * i as f64 / nt as f64;
        values.push(10.0 * t.sin().max(1e-3).log10());
      }
    }
    let m = mesh_from_grid((nt, np), &values, 0.0);
    assert!(!m.is_empty());
    for n in &m.normals {
      assert!(n.iter().all(|v| v.is_finite()), "法线出现非有限值：{n:?}");
      assert!((len(*n) - 1.0).abs() < 1e-3, "法线未归一化：{n:?}");
    }
  }

  #[test]
  fn wireframe_stays_on_the_unit_sphere() {
    let w = unit_sphere_wireframe(3, 8, 24);
    assert_eq!(w.len() % 2, 0, "线框必须是成对的端点");
    assert!(!w.is_empty());
    for p in &w {
      assert!(
        (len(*p) - 1.0).abs() < 1e-5,
        "参考球顶点不在单位球上：{p:?}"
      );
    }
    // 每条线都要有实长（不能退化成点）。
    let mut short = 0;
    for pair in w.as_chunks::<2>().0 {
      let d = [
        pair[1][0] - pair[0][0],
        pair[1][1] - pair[0][1],
        pair[1][2] - pair[0][2],
      ];
      if len(d) < 1e-6 {
        short += 1;
      }
    }
    assert_eq!(short, 0, "有 {short} 段线框退化成点");
  }
}
