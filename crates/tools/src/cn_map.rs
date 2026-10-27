//! 由 Natural Earth 1:50m 州省 GeoJSON 生成中国省级边界 `public/cn-provinces.bin`。
//!
//! 分区号地图只需要中国这一个国家：110m 的国界数据没有省级边界，所以另取 50m 的
//! admin-1，筛出 `adm0_a3 = CHN` 的 31 个要素，按 ISO 3166-2 地名码与
//! [`ham_web_core::reference::CHINA_PROVINCES`] 对齐（省份名与分区号在那张手工表里，
//! 这里只负责几何）。
//!
//! **先校验、后写盘**：省份表里的任何一个省没有几何、或某个省缺标注点属性，都直接失败，
//! 产物与来源戳一个字都不写 —— `data-sync` 的 `regenerate` 会把产物提交进 PR，缺一块的
//! 半成品不能落盘。对不上的要素（表里没有这个码）只打印报告，供人决定要不要补进省份表。
//!
//! 港澳台不在这套分区号里（呼号归其他 DXCC 实体），因此也不进产物。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use ham_web_core::province_map::{ProvinceShape, ProvinceShapes, encode_binary};
use ham_web_core::reference::CHINA_PROVINCES;

use crate::geojson::{self, FeatureCollection, prepared_rings, prop_f64, prop_string};
use crate::provenance;

/// 简化容差（度）：50m 原始数据约 7900 点，0.05° 后约 3300 点（产物 ~7 KB）。
const TOL: f64 = 0.05;

/// Natural Earth 1:50m 州省 GeoJSON 默认下载地址。
pub const DEFAULT_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_admin_1_states_provinces.geojson";

/// 省份表要求的产物路径。
const ARTIFACT: &str = "public/cn-provinces.bin";

/// 一次解析的结果：能进产物的省，以及被跳过的要素说明。
#[derive(Debug)]
struct Collected {
  /// 按地名码排序，输出稳定。
  shapes: Vec<ProvinceShape>,
  /// 属于中国、但没能采纳的要素（给人看的说明）。
  skipped: Vec<String>,
}

/// 从 GeoJSON 里挑出省份表里的 31 个省，**不碰磁盘**。
///
/// 下面几种情况直接失败（不是只打印）：省份表里的省没有对应要素 / 没有可用的外环、
/// 要素缺 `longitude` / `latitude`（标注点没有可靠的退路，退回「第一个环的首点」会落在
/// 省界上）、同一个地名码出现两次。
fn collect_provinces(fc: &FeatureCollection) -> Result<Collected> {
  let mut shapes: Vec<ProvinceShape> = Vec::new();
  let mut matched: HashSet<String> = HashSet::new();
  let mut skipped: Vec<String> = Vec::new();
  let mut problems: Vec<String> = Vec::new();

  for feature in &fc.features {
    let props = &feature.properties;
    if prop_string(props, "adm0_a3").as_deref() != Some("CHN") {
      continue;
    }
    let name = prop_string(props, "name").unwrap_or_else(|| "<unnamed>".to_owned());
    // `iso_3166_2` 形如 `CN-BJ`；只要地名码后缀。
    let Some(code) = prop_string(props, "iso_3166_2")
      .and_then(|iso| iso.split_once('-').map(|(_, suffix)| suffix.to_owned()))
    else {
      skipped.push(format!("{name}（缺少 iso_3166_2）"));
      continue;
    };
    if !CHINA_PROVINCES.iter().any(|p| p.code == code) {
      skipped.push(format!("{code}（省份表里没有这个码：{name}）"));
      continue;
    }
    if matched.contains(&code) {
      problems.push(format!("{code}（{name}）在上游里出现了不止一次"));
      continue;
    }

    let rings = prepared_rings(&feature.geometry, TOL);
    if rings.is_empty() {
      problems.push(format!("{code}（{name}）没有可用的外环"));
      continue;
    }
    // 标注点用 Natural Earth 的 label point：比几何重心靠谱（内蒙古这种狭长形状的重心常
    // 落在省外）。缺了不退到几何上的点 —— 那是边界上的点，标签会压在省界上而没人发现。
    let (Some(lon), Some(lat)) = (prop_f64(props, "longitude"), prop_f64(props, "latitude")) else {
      problems.push(format!("{code}（{name}）缺少 longitude / latitude 标注点"));
      continue;
    };

    matched.insert(code.clone());
    shapes.push(ProvinceShape {
      code,
      label: (lon, lat),
      polys: rings,
    });
  }

  let missing: Vec<&str> = CHINA_PROVINCES
    .iter()
    .filter(|p| !matched.contains(p.code))
    // 已经因为别的原因记过问题的省，不再重复报「没找到」。
    .filter(|p| !problems.iter().any(|m| m.starts_with(p.code)))
    .map(|p| p.name)
    .collect();
  if !missing.is_empty() {
    problems.push(format!(
      "省份表里有、上游里没有的省（{} 个）：{}",
      missing.len(),
      missing.join("、")
    ));
  }
  if !problems.is_empty() {
    bail!(
      "中国省级边界不完整，未写任何文件：\n  - {}",
      problems.join("\n  - ")
    );
  }

  shapes.sort_by(|a, b| a.code.cmp(&b.code));
  Ok(Collected { shapes, skipped })
}

/// 生成 `public/cn-provinces.bin` 与它的来源戳。
pub fn generate(root: &Path, input: Option<&Path>, url: &str) -> Result<()> {
  let (fc, stamp) = geojson::load(input, url)?;
  // 先把全部校验做完，再碰磁盘。
  let Collected { shapes, skipped } = collect_provinces(&fc)?;
  let out = ProvinceShapes { shapes };
  let bin = encode_binary(&out);

  let path = root.join(ARTIFACT);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir)?;
  }
  fs::write(&path, &bin).with_context(|| format!("failed to write {}", path.display()))?;
  provenance::write_sidecar(root, &path, &stamp)?;

  println!(
    "Wrote {} ({} provinces, {} bytes)",
    path.display(),
    out.shapes.len(),
    bin.len()
  );
  if !skipped.is_empty() {
    println!(
      "\n未采纳的中国要素（{} 个；港澳台按设计不收，其余请补进省份表 `CHINA_PROVINCES`）：",
      skipped.len()
    );
    for s in &skipped {
      println!("  - {s}");
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use ham_web_core::province_map::{
    CN_MAP_HEIGHT, CN_MAP_WIDTH, LEADER_MAX_PX, LEADER_MIN_PX, ProvinceShapes as Shapes,
    decode_binary, label_anchor, leader_length_px, project_cn,
  };

  // —— 合成 GeoJSON ——

  /// 一个以 `(lon, lat)` 为中心、边长 2° 的方块省。
  fn square(lon: f64, lat: f64) -> serde_json::Value {
    serde_json::json!({
      "type": "Polygon",
      "coordinates": [[
        [lon - 1.0, lat - 1.0], [lon - 1.0, lat + 1.0],
        [lon + 1.0, lat + 1.0], [lon + 1.0, lat - 1.0], [lon - 1.0, lat - 1.0]
      ]]
    })
  }

  fn feature(adm0: &str, iso: &str, name: &str, lon: f64, lat: f64) -> serde_json::Value {
    serde_json::json!({
      "properties": {
        "adm0_a3": adm0, "iso_3166_2": iso, "name": name, "longitude": lon, "latitude": lat
      },
      "geometry": square(lon, lat)
    })
  }

  /// 省份表里 31 个省各一块，彼此错开。
  fn all_features() -> Vec<serde_json::Value> {
    CHINA_PROVINCES
      .iter()
      .enumerate()
      .map(|(i, p)| {
        let (lon, lat) = (75.0 + (i % 8) as f64 * 7.0, 20.0 + (i / 8) as f64 * 8.0);
        feature("CHN", &format!("CN-{}", p.code), p.name, lon, lat)
      })
      .collect()
  }

  fn collection(features: Vec<serde_json::Value>) -> FeatureCollection {
    serde_json::from_value(serde_json::json!({ "features": features })).expect("合成 GeoJSON")
  }

  fn tmp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("cn-map-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");
    root
  }

  // —— collect_provinces ——

  #[test]
  fn picks_exactly_the_province_table_sorted_by_code() {
    let got = collect_provinces(&collection(all_features())).expect("完整输入应成功");
    let codes: Vec<&str> = got.shapes.iter().map(|s| s.code.as_str()).collect();
    let mut expected: Vec<&str> = CHINA_PROVINCES.iter().map(|p| p.code).collect();
    expected.sort_unstable();
    assert_eq!(codes, expected);
    assert!(got.skipped.is_empty());
  }

  #[test]
  fn ignores_features_of_other_countries() {
    let mut features = all_features();
    features.push(feature("USA", "US-CA", "California", -120.0, 37.0));
    let got = collect_provinces(&collection(features)).expect("其他国家的要素不该影响结果");
    assert_eq!(got.shapes.len(), CHINA_PROVINCES.len());
    assert!(got.skipped.is_empty());
  }

  #[test]
  fn reports_chinese_features_outside_the_table_without_failing() {
    // 港澳台按设计不收：只进「未采纳」报告，不算失败。
    let mut features = all_features();
    features.push(feature("CHN", "CN-TW", "Taiwan", 121.0, 24.0));
    let got = collect_provinces(&collection(features)).expect("表外要素不该让生成失败");
    assert_eq!(got.shapes.len(), CHINA_PROVINCES.len());
    assert_eq!(got.skipped.len(), 1);
    assert!(got.skipped[0].contains("TW"), "{:?}", got.skipped);
  }

  #[test]
  fn fails_and_names_the_province_missing_from_upstream() {
    let features: Vec<_> = all_features()
      .into_iter()
      .filter(|f| f["properties"]["iso_3166_2"] != "CN-HI")
      .collect();
    let err = collect_provinces(&collection(features)).expect_err("缺一个省必须失败");
    let msg = err.to_string();
    assert!(msg.contains("未写任何文件"), "{msg}");
    assert!(msg.contains("海南"), "要点出缺的是哪个省：{msg}");
  }

  #[test]
  fn fails_when_a_label_point_is_missing_instead_of_falling_back_to_the_boundary() {
    let mut features = all_features();
    features[0]["properties"]
      .as_object_mut()
      .expect("props")
      .remove("longitude");
    let code = CHINA_PROVINCES[0].code;
    let err = collect_provinces(&collection(features)).expect_err("缺标注点必须失败");
    let msg = err.to_string();
    assert!(msg.contains(code) && msg.contains("标注点"), "{msg}");
    // 这个省只报一次原因，不再叠一条「上游里没有」。
    assert!(!msg.contains("上游里没有的省"), "{msg}");
  }

  #[test]
  fn fails_on_a_duplicated_code_and_on_a_degenerate_geometry() {
    let mut features = all_features();
    features.push(features[0].clone());
    let msg = collect_provinces(&collection(features))
      .expect_err("重复的地名码必须失败")
      .to_string();
    assert!(msg.contains("不止一次"), "{msg}");

    let mut features = all_features();
    features[1]["geometry"] = serde_json::json!({ "type": "Point", "coordinates": [100.0, 30.0] });
    let msg = collect_provinces(&collection(features))
      .expect_err("没有外环必须失败")
      .to_string();
    assert!(msg.contains("没有可用的外环"), "{msg}");
  }

  // —— generate：先校验、后写盘 ——

  #[test]
  fn writes_nothing_when_validation_fails() {
    let root = tmp_root("incomplete");
    let input = root.join("partial.geojson");
    let partial =
      serde_json::json!({ "features": [feature("CHN", "CN-BJ", "Beijing", 116.4, 39.9)] });
    fs::write(&input, partial.to_string()).expect("write input");

    let err = generate(&root, Some(&input), "unused").expect_err("缺 30 个省必须失败");
    assert!(err.to_string().contains("未写任何文件"), "{err}");
    assert!(!root.join(ARTIFACT).exists(), "校验失败时不该留下产物");
    assert!(
      provenance::read_sidecar(&root, &root.join(ARTIFACT)).is_none(),
      "校验失败时不该留下来源戳"
    );
    let _ = fs::remove_dir_all(&root);
  }

  #[test]
  fn writes_artifact_and_sidecar_that_decode_back_when_complete() {
    let root = tmp_root("complete");
    let input = root.join("full.geojson");
    fs::write(
      &input,
      serde_json::json!({ "features": all_features() }).to_string(),
    )
    .expect("write input");

    generate(&root, Some(&input), "unused").expect("完整输入应成功");
    let bytes = fs::read(root.join(ARTIFACT)).expect("产物");
    let back = decode_binary(&bytes).expect("产物能解码");
    assert_eq!(back.shapes.len(), CHINA_PROVINCES.len());
    let stamp = provenance::read_sidecar(&root, &root.join(ARTIFACT)).expect("来源戳");
    assert!(
      stamp.source.starts_with("file://"),
      "本地输入记成 file://：{}",
      stamp.source
    );
    let _ = fs::remove_dir_all(&root);
  }

  // —— 已提交的产物 ——

  /// 点在多边形内（射线法）。只用于测试里核对标注点。
  fn ring_contains(ring: &[(f64, f64)], (x, y): (f64, f64)) -> bool {
    let mut inside = false;
    for (i, &(x1, y1)) in ring.iter().enumerate() {
      let (x2, y2) = ring[(i + 1) % ring.len()];
      if (y1 > y) != (y2 > y) && x < x1 + (y - y1) * (x2 - x1) / (y2 - y1) {
        inside = !inside;
      }
    }
    inside
  }

  fn contains(shape: &ProvinceShape, p: (f64, f64)) -> bool {
    shape.polys.iter().any(|ring| ring_contains(ring, p))
  }

  fn committed() -> (Vec<u8>, Shapes) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join(ARTIFACT);
    let bytes = fs::read(&path)
      .unwrap_or_else(|e| panic!("读不到 {}（{e}）；先跑 cargo make cn-map", path.display()));
    let shapes = decode_binary(&bytes).expect("cn-provinces.bin 解码失败");
    (bytes, shapes)
  }

  /// 产物与省份表必须一一对应：表里改了码 / 加了省却忘了重建，界面上就会缺一块。
  /// 这里读的是仓库里已提交的产物（离线，不需要联网）。
  #[test]
  fn committed_artifact_covers_exactly_the_province_table() {
    let (_, shapes) = committed();

    let mut codes: Vec<&str> = shapes.shapes.iter().map(|s| s.code.as_str()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), shapes.shapes.len(), "产物里有重复的地名码");

    let mut expected: Vec<&str> = CHINA_PROVINCES.iter().map(|p| p.code).collect();
    expected.sort_unstable();
    assert_eq!(
      codes, expected,
      "产物与省份表不一致：改完省份表跑一遍 cargo make cn-map"
    );

    // 几何要点：每个省至少一个环、环里至少 3 个点。
    for s in &shapes.shapes {
      assert!(!s.polys.is_empty(), "{} 没有环", s.code);
      for ring in &s.polys {
        assert!(ring.len() >= 3, "{} 有一个环少于 3 个点", s.code);
      }
    }
  }

  /// 标注点是地图上的文字落点：普通省份原地标注，所以标注点必须真的在本省几何内；
  /// 写不下字的直辖市把文字推到省外并画引线，引线长度必须落在读得跟上的范围内。
  /// 合成样本测不出「真实产物重新生成后标注点漂移」，所以用已提交的产物守着。
  #[test]
  fn committed_label_points_sit_inside_their_province_or_on_a_short_leader() {
    let (_, shapes) = committed();
    for s in &shapes.shapes {
      let anchor = label_anchor(s);
      match anchor.leader_to {
        None => assert!(
          contains(s, s.label),
          "{} 的标注点 {:?} 不在本省几何内（省名会写到别的省上）",
          s.code,
          s.label
        ),
        Some(target) => {
          assert!(
            contains(s, target),
            "{} 的引线终点 {target:?} 不在本省几何内",
            s.code
          );
          assert!(
            !contains(s, anchor.at),
            "{} 带引线就说明文字在省外，但锚点 {:?} 落在了省内",
            s.code,
            anchor.at
          );
          let gap = leader_length_px(&anchor).expect("带引线的锚点必有长度");
          assert!(
            (LEADER_MIN_PX..LEADER_MAX_PX).contains(&gap),
            "{} 的引线长 {gap:.1} px：太短会盖住省，太长读者跟丢",
            s.code
          );
        }
      }
      // 文字块（省名 + 分区号）不能出视窗。
      let (x, y) = project_cn(anchor.at.0, anchor.at.1);
      assert!(
        (0.0..=CN_MAP_WIDTH).contains(&x) && (0.0..=CN_MAP_HEIGHT).contains(&y),
        "{} 的标注落在视窗外：({x:.1}, {y:.1})",
        s.code
      );
    }
  }

  /// 产物是前端要整份下载的：简化没生效（或误用了 110m 之外更细的数据）体积会翻倍。
  /// 现状约 7 KB，上限留 2 倍多余量。
  #[test]
  fn committed_artifact_stays_small() {
    let (bytes, _) = committed();
    assert!(
      bytes.len() < 16 * 1024,
      "cn-provinces.bin 有 {} 字节：简化是否没生效？",
      bytes.len()
    );
  }
}
