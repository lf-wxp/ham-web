//! 由 Natural Earth 国界 GeoJSON 生成 DXCC 实体边界 `public/dxcc-entities.bin`。
//!
//! 匹配策略：
//! 1. 读取 [`ham_web_core::dxcc`] 内置的实体表（`crates/core/data/dxcc.txt`），构建
//!    「英文名归一化 → DXCC 编号」索引，编号全部来自项目已有数据，不在本文件硬编码；
//! 2. Natural Earth 每个国家要素按 `ADMIN` 国名归一化后精确匹配，个别国名不一致的
//!    用 [`NAME_ALIASES`] 做人工别名；
//! 3. 多边形只取外环（忽略内环湖洞），用 Douglas–Peucker 简化后输出；
//! 4. 未匹配要素与「有实体无几何」的缺口会打印报告，供后续补映射。
//!
//! 注意：Natural Earth 110m 仅到主权国家级别，因此阿拉斯加（KL7）、夏威夷（KH6）
//! 等「一国多实体」的细分需后续叠加 admin-1 州级数据补充。

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use ham_web_core::dxcc_map::{DxccShape, DxccShapes, encode_binary, simplify_ring};

/// 简化容差（度）：越小越精细、体积越大。
const TOL: f64 = 0.25;

/// Natural Earth 1:110m 国界 GeoJSON 默认下载地址。
pub const DEFAULT_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_0_countries.geojson";

/// Natural Earth `ADMIN` 国名 → cty.csv 英文名（仅在归一化后仍对不上时使用）。
///
/// 「一国多实体」的整体国界近似映射到主实体（如俄罗斯→欧洲部分、英国→英格兰、
/// 马来西亚→西马、土耳其→亚洲部分），细分子实体后续叠加 admin-1 州级数据补充。
const NAME_ALIASES: &[(&str, &str)] = &[
  ("United States of America", "United States"),
  ("Dominican Rep.", "Dominican Republic"),
  ("Czechia", "Czech Republic"),
  ("Central African Rep.", "Central African Republic"),
  ("Eq. Guinea", "Equatorial Guinea"),
  ("eSwatini", "Kingdom of Eswatini"),
  ("S. Sudan", "Republic of South Sudan"),
  ("South Sudan", "Republic of South Sudan"),
  ("United Republic of Tanzania", "Tanzania"),
  ("Democratic Republic of the Congo", "Dem. Rep. of the Congo"),
  ("The Bahamas", "Bahamas"),
  ("East Timor", "Timor - Leste"),
  ("Ivory Coast", "Cote d'Ivoire"),
  ("Gambia", "The Gambia"),
  ("North Korea", "DPR of Korea"),
  ("South Korea", "Republic of Korea"),
  ("Germany", "Fed. Rep. of Germany"),
  ("Brunei", "Brunei Darussalam"),
  ("Slovakia", "Slovak Republic"),
  ("Bosnia and Herzegovina", "Bosnia-Herzegovina"),
  ("Republic of Serbia", "Serbia"),
  ("Kosovo", "Republic of Kosovo"),
  ("Trinidad and Tobago", "Trinidad & Tobago"),
  ("Russia", "European Russia"),
  ("Turkey", "Asiatic Turkey"),
  ("United Kingdom", "England"),
  ("Malaysia", "West Malaysia"),
];

// ---- GeoJSON 结构 ----

#[derive(Deserialize)]
struct FeatureCollection {
  features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
  properties: serde_json::Map<String, serde_json::Value>,
  geometry: Geometry,
}

#[derive(Deserialize)]
struct Geometry {
  #[serde(rename = "type")]
  ty: String,
  coordinates: serde_json::Value,
}

fn prop_string(props: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
  props.get(key).and_then(|v| v.as_str()).map(str::to_owned)
}

/// 归一化英文名：仅保留字母数字并转小写，用于模糊匹配国名差异。
fn normalize(s: &str) -> String {
  s.chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

/// 由 cty 实体表构建「归一化英文名 → DXCC 编号」索引。
fn name_index() -> HashMap<String, u16> {
  let mut map = HashMap::new();
  for e in ham_web_core::dxcc::entities() {
    map.insert(normalize(e.name_en), e.dxcc);
  }
  map
}

/// 把 Natural Earth 国名解析为 DXCC 编号。
fn resolve_dxcc(name: &str, index: &HashMap<String, u16>) -> Option<u16> {
  let norm = normalize(name);
  index.get(&norm).copied().or_else(|| {
    let alias = NAME_ALIASES.iter().find(|(n, _)| normalize(n) == norm)?;
    index.get(&normalize(alias.1)).copied()
  })
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

/// 把坐标量化到 0.1°（1 位小数）并去除连续重复点，减小 JSON 体积。
/// 110m 数据本身是粗粒度，0.1°（约 11km）精度对世界地图着色足够。
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

/// 由旧版 JSON（[`DxccShapes`] 结构）直接转换为二进制，无需重新下载 GeoJSON。
pub fn convert_json_to_bin(root: &Path, json_path: &Path) -> Result<()> {
  let text = fs::read_to_string(json_path)
    .with_context(|| format!("failed to read {}", json_path.display()))?;
  let shapes: DxccShapes = serde_json::from_str(&text).context("invalid DxccShapes JSON")?;
  let bin = encode_binary(&shapes);
  let path = root.join("public/dxcc-entities.bin");
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir)?;
  }
  fs::write(&path, &bin).with_context(|| format!("failed to write {}", path.display()))?;
  println!(
    "Converted {} → {} ({} entities, {} bytes)",
    json_path.display(),
    path.display(),
    shapes.shapes.len(),
    bin.len()
  );
  Ok(())
}

/// 生成 `public/dxcc-entities.bin`。
pub fn generate(root: &Path, input: Option<&Path>, url: &str) -> Result<()> {
  // 旧版 JSON 直接迁移为二进制。
  if let Some(p) = input
    && p.extension().and_then(|e| e.to_str()) == Some("json")
  {
    return convert_json_to_bin(root, p);
  }
  let text = match input {
    Some(p) => fs::read_to_string(p).with_context(|| format!("failed to read {}", p.display()))?,
    None => {
      let mut res = ureq::get(url)
        .call()
        .with_context(|| format!("failed to download {url}"))?;
      res
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .with_context(|| format!("failed to download {url}"))?
    }
  };

  let fc: FeatureCollection = serde_json::from_str(&text).context("invalid GeoJSON")?;
  let index = name_index();

  let mut shapes: Vec<DxccShape> = Vec::new();
  let mut seen: HashSet<u16> = HashSet::new();
  let mut unmapped: Vec<String> = Vec::new();

  for feature in &fc.features {
    let name = prop_string(&feature.properties, "ADMIN")
      .or_else(|| prop_string(&feature.properties, "NAME"))
      .unwrap_or_else(|| "<unnamed>".to_owned());
    let iso = prop_string(&feature.properties, "ADM0_A3")
      .or_else(|| prop_string(&feature.properties, "ISO_A3"));
    let Some(dxcc) = resolve_dxcc(&name, &index) else {
      let mut line = name.clone();
      if let Some(iso) = iso {
        let _ = write!(line, " ({iso})");
      }
      unmapped.push(line);
      continue;
    };

    let rings: Vec<Vec<(f64, f64)>> = outer_rings(&feature.geometry)
      .into_iter()
      .map(|r| simplify_ring(&r, TOL))
      .map(quantize_ring)
      .filter(|r| r.len() >= 3)
      .collect();
    if rings.is_empty() {
      continue;
    }

    seen.insert(dxcc);
    match shapes.iter_mut().find(|s| s.dxcc == dxcc) {
      Some(existing) => existing.polys.extend(rings),
      None => shapes.push(DxccShape { dxcc, polys: rings }),
    }
  }

  shapes.sort_by_key(|s| s.dxcc);
  let out = DxccShapes { shapes };
  let path = root.join("public/dxcc-entities.bin");
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir)?;
  }
  let bin = encode_binary(&out);
  fs::write(&path, &bin).with_context(|| format!("failed to write {}", path.display()))?;

  // 报告：未映射要素 + 无几何实体。
  println!(
    "Wrote {} ({} entities, {} bytes)",
    path.display(),
    out.shapes.len(),
    bin.len()
  );
  if !unmapped.is_empty() {
    println!(
      "\n未映射的国界要素（{} 个，需补 NAME_ALIASES 或叠加州级数据）：",
      unmapped.len()
    );
    for name in &unmapped {
      println!("  - {name}");
    }
  }
  let missing: Vec<String> = ham_web_core::dxcc::entities()
    .iter()
    .filter(|e| !seen.contains(&e.dxcc))
    .map(|e| format!("{} {}", e.dxcc, e.name_en))
    .collect();
  if !missing.is_empty() {
    println!(
      "\n有实体但无国界几何（{} 个，多为岛屿/细分子实体）：",
      missing.len()
    );
    for m in missing {
      println!("  - {m}");
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn mock_index() -> HashMap<String, u16> {
    HashMap::from([
      (normalize("China"), 318),
      (normalize("United States"), 291),
      (normalize("Japan"), 339),
    ])
  }

  #[test]
  fn resolves_by_name_and_alias() {
    let idx = mock_index();
    assert_eq!(resolve_dxcc("China", &idx), Some(318));
    assert_eq!(resolve_dxcc("United States of America", &idx), Some(291));
    assert_eq!(resolve_dxcc("Nowhere", &idx), None);
  }

  #[test]
  fn extracts_outer_rings() {
    let geom = Geometry {
      ty: "MultiPolygon".into(),
      coordinates: serde_json::json!([[
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]],
        [[0.1, 0.1], [0.2, 0.1], [0.2, 0.2]] // 内环应被忽略
      ]]),
    };
    let rings = outer_rings(&geom);
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].len(), 4);
  }

  #[test]
  fn simplify_filters_tiny_rings() {
    let ring = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)];
    let simplified = simplify_ring(&ring, 0.25);
    assert!(simplified.len() >= 3);
  }
}
