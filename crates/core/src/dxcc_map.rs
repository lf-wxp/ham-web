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

/// 编码一个环：`u16 点数 · i16 首点 lon×10 · i16 首点 lat×10 · 其后每点 zigzag-varint Δlon/Δlat`。
///
/// 供 [`encode_binary`] 与 [`crate::province_map`] 共用：两种产物只在「环外面」的字段不同，
/// 环本身的格式只写这一份。
///
/// # Panics
/// 环的点数超过 `u16::MAX` 时 panic（生成器的输入是简化后的几百个点，超限是上游数据异常）。
pub(crate) fn encode_ring(ring: &[(f64, f64)], out: &mut Vec<u8>) {
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
      encode_varint(x - prev.0, out);
      encode_varint(y - prev.1, out);
    }
    prev = (x, y);
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
      encode_ring(ring, &mut out);
    }
  }
  out
}

pub(crate) fn read_u8(bytes: &[u8], pos: &mut usize) -> Option<u8> {
  let b = *bytes.get(*pos)?;
  *pos += 1;
  Some(b)
}

pub(crate) fn read_u16(bytes: &[u8], pos: &mut usize) -> Option<u16> {
  Some((u16::from(read_u8(bytes, pos)?) << 8) | u16::from(read_u8(bytes, pos)?))
}

/// 读一个大端 `i16`（首点坐标用）。
pub(crate) fn read_i16(bytes: &[u8], pos: &mut usize) -> Option<i16> {
  read_u16(bytes, pos).map(|v| v as i16)
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

/// 解码一个环（[`encode_ring`] 的逆）；数据截断或累加越界返回 `None`。
///
/// 累加用 `checked_add`：构造出来的超大 delta 会让 `i32` 溢出，debug 下 panic、release 下
/// 静默回绕成别处的点 —— 与「损坏就返回 `None`」的契约不符。
pub(crate) fn decode_ring(bytes: &[u8], pos: &mut usize) -> Option<Vec<(f64, f64)>> {
  let point_count = usize::from(read_u16(bytes, pos)?);
  let mut ring = Vec::with_capacity(point_count);
  let mut prev = (0i32, 0i32);
  for i in 0..point_count {
    let (x, y) = if i == 0 {
      let x = i32::from(read_i16(bytes, pos)?);
      let y = i32::from(read_i16(bytes, pos)?);
      (x, y)
    } else {
      let dx = read_varint(bytes, pos)?;
      let dy = read_varint(bytes, pos)?;
      (prev.0.checked_add(dx)?, prev.1.checked_add(dy)?)
    };
    prev = (x, y);
    ring.push((f64::from(x) / 10.0, f64::from(y) / 10.0));
  }
  Some(ring)
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
      polys.push(decode_ring(bytes, &mut pos)?);
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

  /// 环格式的字节布局是已提交产物的契约：这里钉死一个手算的小例子，重构 / 换实现后
  /// 只要产物字节变了就会红（两个 `.bin` 都靠它）。
  #[test]
  fn ring_encoding_matches_the_documented_layout() {
    // 3 个点：首点 (73.5, 53.5) → i16 735 / 535；之后 Δ=(+5, +1)、(−5, −1)。
    let ring = vec![(73.5, 53.5), (74.0, 53.6), (73.5, 53.5)];
    let mut out = Vec::new();
    encode_ring(&ring, &mut out);
    let mut expected = vec![0, 3]; // u16 点数
    expected.extend(735i16.to_be_bytes());
    expected.extend(535i16.to_be_bytes());
    expected.extend([0x0a, 0x02]); // zigzag(+5)=10、zigzag(+1)=2
    expected.extend([0x09, 0x01]); // zigzag(−5)=9、zigzag(−1)=1
    assert_eq!(out, expected);

    let mut pos = 0;
    assert_eq!(decode_ring(&out, &mut pos), Some(ring));
    assert_eq!(pos, out.len(), "解码要恰好吃完这个环的字节");
  }

  /// 解码契约是「损坏返回 `None`」。构造一个 delta 累加后超出 `i32` 的环：
  /// 修复前 debug 下 panic（`attempt to add with overflow`）、release 下静默回绕成别处的点。
  #[test]
  fn decoding_overflowing_deltas_returns_none_instead_of_panicking() {
    let mut bytes = vec![0, 3, 0, 0, 0, 0]; // 3 个点，首点 (0, 0)
    for _ in 0..2 {
      encode_varint(i32::MAX, &mut bytes); // Δlon：两次累加越过 i32::MAX
      encode_varint(0, &mut bytes);
    }
    let mut pos = 0;
    assert_eq!(decode_ring(&bytes, &mut pos), None);

    // 负方向同理。
    let mut bytes = vec![0, 3, 0, 0, 0, 0];
    for _ in 0..2 {
      encode_varint(i32::MIN, &mut bytes);
      encode_varint(0, &mut bytes);
    }
    let mut pos = 0;
    assert_eq!(decode_ring(&bytes, &mut pos), None);

    // 整份二进制里也一样（version 1、1 个实体 dxcc=1、1 个环）。
    let mut whole = vec![1, 0, 1, 0, 1, 1];
    whole.extend(&bytes);
    assert!(decode_binary(&whole).is_none());
  }

  #[test]
  fn truncated_rings_return_none() {
    let mut out = Vec::new();
    encode_ring(&[(1.0, 2.0), (1.5, 2.5), (2.0, 3.0)], &mut out);
    for cut in 0..out.len() {
      let mut pos = 0;
      assert_eq!(decode_ring(&out[..cut], &mut pos), None, "截到 {cut} 字节");
    }
  }
}
