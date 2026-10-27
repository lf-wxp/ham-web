//! 中国省级边界多边形：分区号地图（考试速查页）用的几何数据。
//!
//! 数据由 `ham-web-tools cn-map` 从 Natural Earth 1:50m 州省 GeoJSON 生成，
//! 输出到 `public/cn-provinces.bin`（i16 定点 + delta 二进制，约 7 KB），前端按需
//! 加载，不进 wasm。省份的中文名与分区号不在这里：那部分是
//! [`crate::reference::CHINA_PROVINCES`] 的手工表，两边用 ISO 地名码（`BJ` / `HL` …）对齐。
//!
//! 投影是等距圆柱（与全站地图同族），经度按 36°N 的余弦压缩，形状比值接近真实。

use crate::dxcc_map::{decode_ring, encode_ring, read_i16, read_u8, read_u16};

/// 一个省级行政区的边界。
#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceShape {
  /// ISO 3166-2 的地名码后缀（`CN-BJ` → `BJ`）。
  pub code: String,
  /// 标注的落点 `(经度, 纬度)`：取 Natural Earth 给的 label point，比几何重心靠谱
  /// （内蒙古这种狭长形状的重心常落在省外）。省名与分区号都写在这个点上。
  pub label: (f64, f64),
  /// 一个或多个多边形环（岛屿拆成多个环），每个环是 `(经度, 纬度)` 顶点序列。
  pub polys: Vec<Vec<(f64, f64)>>,
}

/// 全部省级边界，即 `public/cn-provinces.bin` 的顶层内容。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProvinceShapes {
  pub shapes: Vec<ProvinceShape>,
}

impl ProvinceShapes {
  /// 按 ISO 地名码查省级边界。
  #[must_use]
  pub fn shape(&self, code: &str) -> Option<&ProvinceShape> {
    self.shapes.iter().find(|s| s.code == code)
  }
}

// —— 投影 ——

/// 经度每度对应的像素（`10 × cos 36°`，按中国腹地的纬度压缩，形状比值不失真）。
const KX: f64 = 8.1;
/// 纬度每度对应的像素。
const KY: f64 = 10.0;
/// 视窗西边界（东经）。取 73°：新疆最西端 73.6° 留一点余量。
const LON_MIN: f64 = 73.0;
/// 视窗北边界（北纬）。取 54°：漠河最北端 53.6° 留一点余量。
const LAT_MAX: f64 = 54.0;
/// 视窗覆盖的经度跨度（度）：73°E → 135°E，最东端黑龙江 134.8° 留一点余量。
const LON_SPAN: f64 = 62.0;
/// 视窗覆盖的纬度跨度（度）：54°N → 17.5°N，最南端海南 18.2° 留一点余量。
const LAT_SPAN: f64 = 36.5;

/// 地图视窗宽度（SVG 用户单位）。
pub const CN_MAP_WIDTH: f64 = LON_SPAN * KX;
/// 地图视窗高度（SVG 用户单位）。
pub const CN_MAP_HEIGHT: f64 = LAT_SPAN * KY;

/// 省名字号（SVG 用户单位）。汉字两三个字，宽度稳定，写得进多数省界。
pub const LABEL_NAME_SIZE: f64 = 6.8;
/// 分区号字号。比省名小一档，让「名字」与「数字」在视觉上分出主次。
pub const LABEL_DIGIT_SIZE: f64 = 6.2;
/// 分区号基线比省名基线低多少（略大于字号：两行之间留出一点空隙）。
pub const LABEL_LINE_GAP: f64 = 7.2;
/// 引线从文字块边缘出发的半径（≈ 半个文本块）：否则线会从省名与分区号中间穿过去。
pub const LEADER_START_RADIUS: f64 = 9.0;

/// 经纬度 → SVG 坐标（等距圆柱，经度按中纬度压缩）。
#[must_use]
pub fn project_cn(lon: f64, lat: f64) -> (f64, f64) {
  ((lon - LON_MIN) * KX, (LAT_MAX - lat) * KY)
}

/// 地图上的省名标注位置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelAnchor {
  /// 文本锚点 `(经度, 纬度)`：省名以它为中心，分区号写在它下面一行。
  pub at: (f64, f64),
  /// 需要引线时给出引线终点（省份自己的 label point）。
  pub leader_to: Option<(f64, f64)>,
}

/// 每个省的标注锚点。
///
/// 默认就落在 Natural Earth 的 label point 上（它比几何重心靠谱：内蒙古这类狭长形状的
/// 重心常落在省外）。北京 / 天津 / 上海例外 —— 面积小到两个汉字都写不进省界（北京的东西
/// 宽度投影后约 10 px，而「北京」两个字要 14 px），锚点推到省外最近的空白处并画一条细引线
/// 指回省界内，否则要么字压省界糊成一团，要么读者把字当成隔壁省的。
#[must_use]
pub fn label_anchor(shape: &ProvinceShape) -> LabelAnchor {
  match shape.code.as_str() {
    // 正北的河北北端（围场一带），引线竖直落回北京。
    "BJ" => LabelAnchor {
      at: (116.40, 41.70),
      leader_to: Some(shape.label),
    },
    // 渤海中部，引线向西落回天津。
    "TJ" => LabelAnchor {
      at: (119.60, 38.60),
      leader_to: Some(shape.label),
    },
    // 东海海面，引线向西落回上海。
    "SH" => LabelAnchor {
      at: (123.70, 30.90),
      leader_to: Some(shape.label),
    },
    _ => LabelAnchor {
      at: shape.label,
      leader_to: None,
    },
  }
}

/// 引线的最短长度（SVG 用户单位）：再短，线几乎全被文字块盖住，等于没画。
pub const LEADER_MIN_PX: f64 = 8.0;
/// 引线的最长长度：超过这个值说明文字推得离省太远，读者跟丢了。
pub const LEADER_MAX_PX: f64 = 25.0;

/// 引线在图上的长度（SVG 用户单位）；没有引线返回 `None`。
#[must_use]
pub fn leader_length_px(anchor: &LabelAnchor) -> Option<f64> {
  let target = anchor.leader_to?;
  let (ax, ay) = project_cn(anchor.at.0, anchor.at.1);
  let (tx, ty) = project_cn(target.0, target.1);
  Some(((ax - tx).powi(2) + (ay - ty).powi(2)).sqrt())
}

/// 一个环 → SVG `points` 字符串（用本模块的投影，保留 1 位小数）。
#[must_use]
pub fn ring_points(ring: &[(f64, f64)]) -> String {
  ring
    .iter()
    .map(|&(lon, lat)| {
      let (x, y) = project_cn(lon, lat);
      format!("{x:.1},{y:.1}")
    })
    .collect::<Vec<_>>()
    .join(" ")
}

// —— 紧凑二进制编码 ——
//
// 与 `dxcc-entities.bin` 同一套思路（i16 定点 ×10 + zigzag varint delta），只是多了
// 字符串地名码与标注点；「环」的格式就是同一份实现（[`crate::dxcc_map`] 的 `encode_ring` /
// `decode_ring`）。格式（全部大端）：
//
//   u8    version（=1）
//   u16   省份数
//   每省： u8 码长 · 码字节 · i16 标注经度×10 · i16 标注纬度×10 · u8 环数
//     每环： u16 点数 · i16 首点 lon×10 · i16 首点 lat×10 · 其后每点 zigzag-varint Δlon/Δlat

/// 把几何编码为紧凑二进制。
#[must_use]
pub fn encode_binary(shapes: &ProvinceShapes) -> Vec<u8> {
  let mut out = Vec::with_capacity(shapes.shapes.len() * 32 + 1024);
  out.push(1u8); // version
  out.extend_from_slice(
    &u16::try_from(shapes.shapes.len())
      .expect("province count exceeds u16")
      .to_be_bytes(),
  );
  for s in &shapes.shapes {
    let code = s.code.as_bytes();
    out.push(u8::try_from(code.len()).expect("province code too long"));
    out.extend_from_slice(code);
    out.extend_from_slice(&((s.label.0 * 10.0).round() as i16).to_be_bytes());
    out.extend_from_slice(&((s.label.1 * 10.0).round() as i16).to_be_bytes());
    out.push(u8::try_from(s.polys.len()).expect("ring count exceeds u8"));
    for ring in &s.polys {
      encode_ring(ring, &mut out);
    }
  }
  out
}

/// 从紧凑二进制解码；格式不符或数据损坏返回 `None`。
#[must_use]
pub fn decode_binary(bytes: &[u8]) -> Option<ProvinceShapes> {
  let mut pos = 0usize;
  if read_u8(bytes, &mut pos)? != 1 {
    return None;
  }
  let count = usize::from(read_u16(bytes, &mut pos)?);
  let mut shapes = Vec::with_capacity(count);
  for _ in 0..count {
    let code_len = usize::from(read_u8(bytes, &mut pos)?);
    let code_bytes = bytes.get(pos..pos.checked_add(code_len)?)?;
    let code = std::str::from_utf8(code_bytes).ok()?.to_owned();
    pos += code_len;
    let label_lon = f64::from(read_i16(bytes, &mut pos)?) / 10.0;
    let label_lat = f64::from(read_i16(bytes, &mut pos)?) / 10.0;
    let ring_count = usize::from(read_u8(bytes, &mut pos)?);
    let mut polys = Vec::with_capacity(ring_count);
    for _ in 0..ring_count {
      polys.push(decode_ring(bytes, &mut pos)?);
    }
    shapes.push(ProvinceShape {
      code,
      label: (label_lon, label_lat),
      polys,
    });
  }
  Some(ProvinceShapes { shapes })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> ProvinceShapes {
    ProvinceShapes {
      shapes: vec![
        ProvinceShape {
          code: "BJ".to_owned(),
          label: (116.4, 39.9),
          polys: vec![vec![
            (115.4, 39.4),
            (117.4, 39.4),
            (117.4, 41.0),
            (115.4, 39.4),
          ]],
        },
        ProvinceShape {
          code: "HI".to_owned(),
          label: (109.8, 19.2),
          polys: vec![
            vec![(108.6, 18.2), (111.0, 18.2), (111.0, 20.1), (108.6, 18.2)],
            vec![(109.0, 19.0), (109.5, 19.0), (109.5, 19.5), (109.0, 19.0)],
          ],
        },
      ],
    }
  }

  #[test]
  fn binary_round_trips() {
    let shapes = sample();
    let bin = encode_binary(&shapes);
    let back = decode_binary(&bin).expect("decode");
    assert_eq!(back, shapes);
    assert!(back.shape("BJ").is_some());
    assert!(back.shape("TW").is_none());
  }

  #[test]
  fn binary_is_compact_for_a_small_sample() {
    // 只守「编码没有多余开销」：两个省共 3 个环的小样本远小于 200 字节。
    // 真实产物的体积上限（约 7 KB、不得超过 16 KB）由 `ham-web-tools` 的
    // `cn_map::committed_artifact_stays_small` 守着 —— 简化没生效时前端要白下几十 KB。
    let bin = encode_binary(&sample());
    assert!(bin.len() < 200, "小样本编码后有 {} 字节", bin.len());
  }

  #[test]
  fn binary_rejects_garbage() {
    assert!(decode_binary(&[]).is_none());
    assert!(decode_binary(&[9, 0, 0]).is_none()); // 非法 version
    assert!(decode_binary(&[1, 0, 1]).is_none()); // 声称有 1 个省，数据不够
  }

  #[test]
  fn decoding_rejects_truncation_bad_utf8_and_overflow() {
    let bin = encode_binary(&sample());
    // 任何一个前缀都不是完整产物：全部返回 None，不 panic。
    for cut in 0..bin.len() {
      assert!(
        decode_binary(&bin[..cut]).is_none(),
        "截到 {cut} 字节仍被接受"
      );
    }
    // 地名码不是合法 UTF-8。
    assert!(decode_binary(&[1, 0, 1, 1, 0xff, 0, 0, 0, 0, 0]).is_none());
    // 地名码长度声称超过剩余字节。
    assert!(decode_binary(&[1, 0, 1, 200, b'A']).is_none());
  }

  #[test]
  fn projection_covers_the_viewbox() {
    assert_eq!(project_cn(LON_MIN, LAT_MAX), (0.0, 0.0));
    let (x, y) = project_cn(LON_MIN + LON_SPAN, LAT_MAX - LAT_SPAN);
    assert!((x - CN_MAP_WIDTH).abs() < 1e-9);
    assert!((y - CN_MAP_HEIGHT).abs() < 1e-9);
    // 东移加 x、南移加 y。
    assert!(project_cn(120.0, 30.0).0 > project_cn(100.0, 30.0).0);
    assert!(project_cn(120.0, 20.0).1 > project_cn(120.0, 40.0).1);
  }

  #[test]
  fn label_anchors_put_names_inside_the_province_or_on_a_leader() {
    let shapes = sample();
    // 普通省份：原地标注，不画引线。
    let hi = shapes.shape("HI").expect("HI");
    let plain = label_anchor(hi);
    assert_eq!(plain.at, hi.label);
    assert_eq!(plain.leader_to, None);

    // 直辖市：锚点离开省界，并记住要引回哪里。
    let bj = shapes.shape("BJ").expect("BJ");
    let narrow = label_anchor(bj);
    assert_eq!(narrow.leader_to, Some(bj.label));
    let gap = leader_length_px(&narrow).expect("有引线");
    assert!(
      (LEADER_MIN_PX..LEADER_MAX_PX).contains(&gap),
      "引线长度 {gap:.1} px 不合适"
    );
  }

  #[test]
  fn ring_points_are_svg_ready() {
    let pts = ring_points(&[(73.0, 54.0), (83.0, 44.0)]);
    assert_eq!(pts, "0.0,0.0 81.0,100.0");
  }
}
