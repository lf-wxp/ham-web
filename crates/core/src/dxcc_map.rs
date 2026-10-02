//! DXCC 实体边界多边形：世界地图按实体着色（choropleth）用的几何数据。
//!
//! 数据由 `ham-web-tools dxcc-map` 从 Natural Earth 国界 GeoJSON 生成，输出到
//! `public/dxcc-entities.bin`（i16 定点 + delta 二进制），前端按需加载（不增大 wasm 体积）。
//! 这里只定义运行时结构、几何简化工具与二进制编解码，供前端渲染与构建工具共用。

use serde::{Deserialize, Serialize};

/// 一个 DXCC 实体的边界。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DxccShape {
  /// ADIF DXCC 编号（与 [`crate::dxcc::Entity::dxcc`] 对应）。
  pub dxcc: u16,
  /// 一个或多个多边形环（岛屿实体拆成多个环），
  /// 每个环是 `(经度, 纬度)` 顶点序列（东经为正、北纬为正，闭合）。
  pub polys: Vec<Vec<(f64, f64)>>,
}

/// 全部 DXCC 实体边界，即 `public/dxcc-entities.json` 的顶层结构。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DxccShapes {
  pub shapes: Vec<DxccShape>,
}

impl DxccShapes {
  /// 按 DXCC 编号查找实体边界。
  #[must_use]
  pub fn shape(&self, dxcc: u16) -> Option<&DxccShape> {
    self.shapes.iter().find(|s| s.dxcc == dxcc)
  }
}

/// Douglas–Peucker 折线简化：删除与弦距离小于 `tol`（单位：度）的点，首尾点必保留。
#[must_use]
pub fn simplify_ring(points: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
  if points.len() < 3 {
    return points.to_vec();
  }
  let mut keep = vec![false; points.len()];
  keep[0] = true;
  keep[points.len() - 1] = true;
  let tol2 = tol * tol;
  let mut stack = vec![(0usize, points.len() - 1)];

  while let Some((a, b)) = stack.pop() {
    if b <= a + 1 {
      continue;
    }
    let (ax, ay) = points[a];
    let (bx, by) = points[b];
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    let (mut max_d, mut max_i) = (0.0f64, a);
    for (i, &(px, py)) in points.iter().enumerate().take(b).skip(a + 1) {
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

  points
    .iter()
    .zip(keep.iter())
    .filter_map(|(p, &k)| k.then_some(*p))
    .collect()
}

// —— 紧凑二进制编码 ——
//
// 用 i16 定点（经纬度 ×10）+ 相邻点 zigzag varint delta 编码，替代 JSON 的
// `[[f64,f64], …]` 文本，体积约降 4/5。格式（全部大端）：
//
//   u8   version（=1）
//   u16  实体数
//   每实体： u16 dxcc · u8 环数
//     每环：  u16 点数 · i16 首点 lon×10 · i16 首点 lat×10 · 其后每点 zigzag-varint Δlon/Δlat

/// zigzag + LEB128 变长整数编码（小 delta 1 字节）。
fn encode_varint(v: i32, out: &mut Vec<u8>) {
  let z = ((v << 1) ^ (v >> 31)) as u32;
  let mut x = z;
  loop {
    let b = (x & 0x7f) as u8;
    x >>= 7;
    if x == 0 {
      out.push(b);
      break;
    }
    out.push(b | 0x80);
  }
}

/// 把几何编码为紧凑二进制。
#[must_use]
pub fn encode_binary(shapes: &DxccShapes) -> Vec<u8> {
  let mut out = Vec::with_capacity(shapes.shapes.len() * 16 + 1024);
  out.push(1u8); // version
  out.extend_from_slice(
    &u16::try_from(shapes.shapes.len())
      .expect("shapes count exceeds u16")
      .to_be_bytes(),
  );
  for s in &shapes.shapes {
    out.extend_from_slice(&s.dxcc.to_be_bytes());
    out.push(u8::try_from(s.polys.len()).expect("ring count exceeds u8"));
    for ring in &s.polys {
      out.extend_from_slice(
        &u16::try_from(ring.len())
          .expect("point count exceeds u16")
          .to_be_bytes(),
      );
      let mut prev = (0i32, 0i32);
      for (i, &(lon, lat)) in ring.iter().enumerate() {
        let x = (lon * 10.0).round() as i32;
        let y = (lat * 10.0).round() as i32;
        if i == 0 {
          out.extend_from_slice(&(x as i16).to_be_bytes());
          out.extend_from_slice(&(y as i16).to_be_bytes());
        } else {
          encode_varint(x - prev.0, &mut out);
          encode_varint(y - prev.1, &mut out);
        }
        prev = (x, y);
      }
    }
  }
  out
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Option<u8> {
  let b = *bytes.get(*pos)?;
  *pos += 1;
  Some(b)
}

fn read_u16(bytes: &[u8], pos: &mut usize) -> Option<u16> {
  Some((u16::from(read_u8(bytes, pos)?) << 8) | u16::from(read_u8(bytes, pos)?))
}

/// zigzag + LEB128 解码。
fn read_varint(bytes: &[u8], pos: &mut usize) -> Option<i32> {
  let mut result: u32 = 0;
  let mut shift = 0u32;
  loop {
    let b = read_u8(bytes, pos)?;
    result |= u32::from(b & 0x7f) << shift;
    if b & 0x80 == 0 {
      break;
    }
    shift += 7;
    if shift >= 32 {
      return None;
    }
  }
  Some(((result >> 1) as i32) ^ -((result & 1) as i32))
}

/// 从紧凑二进制解码几何；格式不符或数据损坏返回 `None`。
#[must_use]
pub fn decode_binary(bytes: &[u8]) -> Option<DxccShapes> {
  let mut pos = 0usize;
  if read_u8(bytes, &mut pos)? != 1 {
    return None;
  }
  let count = usize::from(read_u16(bytes, &mut pos)?);
  let mut shapes = Vec::with_capacity(count);
  for _ in 0..count {
    let dxcc = read_u16(bytes, &mut pos)?;
    let ring_count = usize::from(read_u8(bytes, &mut pos)?);
    let mut polys = Vec::with_capacity(ring_count);
    for _ in 0..ring_count {
      let point_count = usize::from(read_u16(bytes, &mut pos)?);
      let mut ring = Vec::with_capacity(point_count);
      let mut prev = (0i32, 0i32);
      for i in 0..point_count {
        let (x, y) = if i == 0 {
          let x = i32::from(read_u16(bytes, &mut pos)? as i16);
          let y = i32::from(read_u16(bytes, &mut pos)? as i16);
          (x, y)
        } else {
          let dx = read_varint(bytes, &mut pos)?;
          let dy = read_varint(bytes, &mut pos)?;
          (prev.0 + dx, prev.1 + dy)
        };
        prev = (x, y);
        ring.push((f64::from(x) / 10.0, f64::from(y) / 10.0));
      }
      polys.push(ring);
    }
    shapes.push(DxccShape { dxcc, polys });
  }
  Some(DxccShapes { shapes })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn simplify_keeps_endpoints_and_corner() {
    let poly = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)];
    assert_eq!(simplify_ring(&poly, 0.5), vec![(0.0, 0.0), (3.0, 0.0)]);

    let corner = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
    assert_eq!(simplify_ring(&corner, 0.1).len(), 3);
  }

  #[test]
  fn shapes_round_trip_and_lookup() {
    let shapes = DxccShapes {
      shapes: vec![DxccShape {
        dxcc: 318,
        polys: vec![vec![(73.0, 53.0), (135.0, 48.0), (73.0, 53.0)]],
      }],
    };
    let json = serde_json::to_string(&shapes).expect("serialize");
    let back: DxccShapes = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, shapes);
    assert!(back.shape(318).is_some());
    assert!(back.shape(1).is_none());
  }

  #[test]
  fn binary_round_trips_and_shrinks() {
    let shapes = DxccShapes {
      shapes: vec![
        DxccShape {
          dxcc: 318,
          polys: vec![
            vec![(73.5, 53.5), (74.0, 53.6), (74.5, 53.7), (73.5, 53.5)],
            vec![(100.0, 20.0), (100.1, 20.0), (100.1, 20.1), (100.0, 20.0)],
          ],
        },
        DxccShape {
          dxcc: 291,
          polys: vec![vec![(-125.0, 30.0), (-124.9, 30.1), (-125.0, 30.0)]],
        },
      ],
    };
    let bin = encode_binary(&shapes);
    let back = decode_binary(&bin).expect("decode");
    assert_eq!(back, shapes);
    // 二进制应显著小于 JSON 文本。
    let json_len = serde_json::to_string(&shapes).expect("json").len();
    assert!(
      bin.len() < json_len,
      "binary {} >= json {}",
      bin.len(),
      json_len
    );
  }

  #[test]
  fn binary_rejects_garbage() {
    assert!(decode_binary(&[]).is_none());
    assert!(decode_binary(&[9, 0, 0]).is_none()); // 非法 version
  }
}
