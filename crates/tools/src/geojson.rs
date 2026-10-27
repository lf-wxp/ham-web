//! Natural Earth GeoJSON 的共用读取与几何预处理。
//!
//! `dxcc-map`（110m 国界）与 `cn-map`（50m 省界）只是「匹配规则」不同（国名 → DXCC 编号 /
//! 地名码 → 省份表）：读本地文件或下载、解析、取外环、简化、量化这些步骤是同一套，
//! 放在这里只写一份。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ham_web_core::dxcc_map::simplify_ring;
use serde::Deserialize;

use crate::provenance::{self, Stamp};

/// 下载上限：Natural Earth 的 GeoJSON 最大也就几十 MB（1:50m 州省约 2 MB），留足余量。
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;

/// 要素的属性表。
pub(crate) type Props = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Deserialize)]
pub(crate) struct FeatureCollection {
  pub(crate) features: Vec<Feature>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Feature {
  pub(crate) properties: Props,
  pub(crate) geometry: Geometry,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Geometry {
  #[serde(rename = "type")]
  ty: String,
  coordinates: serde_json::Value,
}

/// 读入 GeoJSON：给了本地文件就读本地（来源戳记成 `file://…`），否则从 `url` 下载
/// （戳里带上服务器给的校验头，供 `check-data-freshness` 做条件请求）。
pub(crate) fn load(input: Option<&Path>, url: &str) -> Result<(FeatureCollection, Stamp)> {
  match input {
    Some(path) => {
      let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
      let fc = parse(&bytes, &path.display().to_string())?;
      Ok((fc, Stamp::of_local(path, &bytes)))
    }
    None => {
      let fetched = provenance::download(url, MAX_DOWNLOAD_BYTES)?;
      let fc = parse(&fetched.bytes, url)?;
      let stamp = Stamp::of(url, &fetched.bytes)
        .with_validators(fetched.etag.as_deref(), fetched.last_modified.as_deref());
      Ok((fc, stamp))
    }
  }
}

/// 字节 → 要素集合；`origin` 只用于报错时指出是哪一份输入。
fn parse(bytes: &[u8], origin: &str) -> Result<FeatureCollection> {
  let text = std::str::from_utf8(bytes).with_context(|| format!("{origin} 不是 UTF-8"))?;
  serde_json::from_str(text).context("invalid GeoJSON")
}

/// 读属性：先按给定的键，再按全大写的键（admin-0 用大写键，admin-1 用小写键，两种都试）。
fn prop<'a>(props: &'a Props, key: &str) -> Option<&'a serde_json::Value> {
  props
    .get(key)
    .or_else(|| props.get(&key.to_ascii_uppercase()))
}

/// 字符串属性。
pub(crate) fn prop_string(props: &Props, key: &str) -> Option<String> {
  prop(props, key)
    .and_then(serde_json::Value::as_str)
    .map(str::to_owned)
}

/// 数值属性。
pub(crate) fn prop_f64(props: &Props, key: &str) -> Option<f64> {
  prop(props, key).and_then(serde_json::Value::as_f64)
}

/// 两个生成器共用的几何流水线：取外环 → Douglas–Peucker 简化（`tolerance` 单位：度）
/// → 量化到 0.1° → 丢掉退化环（不足 3 点）。
pub(crate) fn prepared_rings(geometry: &Geometry, tolerance: f64) -> Vec<Vec<(f64, f64)>> {
  outer_rings(geometry)
    .into_iter()
    .map(|r| simplify_ring(&r, tolerance))
    .map(quantize_ring)
    .filter(|r| r.len() >= 3)
    .collect()
}

/// 提取 Geometry 的所有外环（`Polygon` 取第一个环，`MultiPolygon` 逐多边形取首环）。
fn outer_rings(geometry: &Geometry) -> Vec<Vec<(f64, f64)>> {
  match geometry.ty.as_str() {
    "Polygon" => polygon_first_ring(&geometry.coordinates)
      .into_iter()
      .collect(),
    "MultiPolygon" => {
      let serde_json::Value::Array(polys) = &geometry.coordinates else {
        return Vec::new();
      };
      polys.iter().filter_map(polygon_first_ring).collect()
    }
    _ => Vec::new(),
  }
}

/// 取单个 Polygon 的首个环（外环）。
fn polygon_first_ring(coords: &serde_json::Value) -> Option<Vec<(f64, f64)>> {
  coords.as_array()?.first().map(ring_to_points)
}

/// 一个环 `[[lon,lat], …]` → 顶点序列。
fn ring_to_points(ring: &serde_json::Value) -> Vec<(f64, f64)> {
  let serde_json::Value::Array(pts) = ring else {
    return Vec::new();
  };
  pts
    .iter()
    .filter_map(|p| {
      let arr = p.as_array()?;
      Some((arr.first()?.as_f64()?, arr.get(1)?.as_f64()?))
    })
    .collect()
}

/// 把坐标量化到 0.1°（1 位小数）并去除连续重复点，减小二进制产物的体积。
/// 0.1°（约 11 km）精度对 110m 世界图与 50m 省界图的着色都足够。
fn quantize_ring(ring: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
  let mut out: Vec<(f64, f64)> = Vec::with_capacity(ring.len());
  for (x, y) in ring {
    let p = ((x * 10.0).round() / 10.0, (y * 10.0).round() / 10.0);
    if out.last() != Some(&p) {
      out.push(p);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn geometry(ty: &str, coordinates: serde_json::Value) -> Geometry {
    Geometry {
      ty: ty.into(),
      coordinates,
    }
  }

  #[test]
  fn extracts_outer_rings() {
    let geom = geometry(
      "MultiPolygon",
      serde_json::json!([[
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]],
        [[0.1, 0.1], [0.2, 0.1], [0.2, 0.2]] // 内环应被忽略
      ]]),
    );
    let rings = outer_rings(&geom);
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].len(), 4);
  }

  #[test]
  fn prepared_rings_keep_valid_rings_and_drop_degenerate_ones() {
    let square = geometry(
      "Polygon",
      serde_json::json!([[[0.0, 0.0], [0.0, 2.0], [2.0, 2.0], [2.0, 0.0], [0.0, 0.0]]]),
    );
    let kept = prepared_rings(&square, 0.25);
    assert_eq!(kept.len(), 1);
    assert!(kept[0].len() >= 3);

    // 整个环都落在同一个 0.1° 格子里：量化后塌成一个点，不成环，必须丢掉。
    let speck = geometry(
      "Polygon",
      serde_json::json!([[[0.0, 0.0], [0.01, 0.01], [0.02, 0.0], [0.0, 0.0]]]),
    );
    assert!(prepared_rings(&speck, 0.0).is_empty());
    // 不认识的几何类型没有外环。
    assert!(prepared_rings(&geometry("Point", serde_json::json!([1.0, 2.0])), 0.1).is_empty());
  }

  #[test]
  fn quantize_rounds_to_a_tenth_and_collapses_repeats() {
    let out = quantize_ring(vec![(1.04, 2.0), (1.01, 2.04), (1.26, 2.0)]);
    assert_eq!(out, vec![(1.0, 2.0), (1.3, 2.0)]);
  }

  #[test]
  fn props_fall_back_to_the_uppercase_key() {
    let props: Props = serde_json::from_value(serde_json::json!({
      "ADM0_A3": "CHN",
      "name": "lower",
      "NAME": "upper",
      "longitude": 116.4,
      "bad": 7
    }))
    .expect("props");
    // admin-0 用大写键、admin-1 用小写键：两种都能读到。
    assert_eq!(prop_string(&props, "adm0_a3").as_deref(), Some("CHN"));
    // 两个键都有时，按给定的键优先。
    assert_eq!(prop_string(&props, "name").as_deref(), Some("lower"));
    assert_eq!(prop_string(&props, "NAME").as_deref(), Some("upper"));
    assert_eq!(prop_f64(&props, "longitude"), Some(116.4));
    // 类型不符与缺失都是 `None`，不 panic。
    assert_eq!(prop_string(&props, "bad"), None);
    assert_eq!(prop_f64(&props, "missing"), None);
  }

  #[test]
  fn parse_names_the_input_when_it_is_not_utf8_or_not_geojson() {
    let err = parse(&[0xff, 0xfe, 0x00], "some.geojson").expect_err("非 UTF-8 应当报错");
    assert!(err.to_string().contains("some.geojson"), "{err}");
    assert!(err.to_string().contains("UTF-8"), "{err}");

    let err = parse(b"{}", "x").expect_err("缺 features 应当报错");
    assert!(err.to_string().contains("invalid GeoJSON"), "{err}");

    let fc = parse(br#"{"features":[]}"#, "x").expect("空集合合法");
    assert!(fc.features.is_empty());
  }
}
