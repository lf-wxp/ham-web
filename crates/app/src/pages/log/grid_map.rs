//! 网格地图绘制方案（Maidenhead Grid Locator 世界地图）。
//!
//! 本模块同时承载「绘制方案」的设计说明与实现，核心决策如下：
//!
//! ## 1. 投影方式：等距圆柱投影（Equirectangular / Plate Carrée）
//! 经纬度线性映射到平面：`x = (lon + 180°) * k`，`y = (90° - lat) * k`，其中 `k = 2`（720×360 画布）。
//! 选择理由：
//! - Maidenhead 网格本身按经纬度等分（field 20°×10°、square 2°×1°、subsquare 5′×2.5′），
//!   在等距圆柱投影下所有网格都是**轴对齐的规则矩形**，几何计算、命中检测、标签居中都退化为简单乘加。
//! - 若改用 Web Mercator，高纬网格会被非线性拉伸、不再是矩形，且 85° 以上区域发散，
//!   与“网格定位器”的语义相悖；等距圆柱是全球网格图的事实标准（QRZ/HamGridMaps 等均采用）。
//! - 代价是高纬面积放大（如格陵兰），但网格图以“格位”而非“面积”为信息载体，可接受。
//!
//! ## 交互能力
//! - 滚轮 / 双指缩放、拖拽平移、双击复位、悬停经纬度提示；
//! - **反子午线环绕**：`viewBox.x` 允许越过 ±180°，世界内容在 x 方向平铺三份，实现全球无缝横拖；
//! - 网格搜索定位（红色十字标记）、波段 / 模式筛选热力、square 悬停 tooltip、
//!   本台网格“家”标记、field / square 两级点击展开、灰线（晨昏圈）叠加开关。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::dxcc::dxcc_entity;
use ham_web_core::frequencies::band_of;
use ham_web_core::grid::{field_index, lat_lon_from_grid, square_index};
use leptos::prelude::*;
use leptos::svg;
use wasm_bindgen::JsValue;

use crate::pages::grayline::{
  band_path, day_of_year, night_path, solar_declination, subsolar_longitude,
};
use crate::ui::{Size, Variant, button_class, input_class};

use super::LogEntry;

/// 等距圆柱投影比例：经度每度 → `2` 像素。
const K: f64 = 2.0;
/// 世界画布宽/高（720×360 = 360°×2 × 180°×2）。
const MAP_W: f64 = 720.0;
const MAP_H: f64 = 360.0;

/// 世界内容在 x 方向的平铺偏移（×MAP_W），用于反子午线环绕：左/中/右三份。
const WORLD_OFFSETS: [f64; 3] = [-1.0, 0.0, 1.0];

/// 波段显示顺序（低频 → 高频），筛选下拉据此排序。
const BAND_ORDER: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

// ---------------------------------------------------------------------------
// ## 2. 陆地边界数据与简化策略
// ---------------------------------------------------------------------------
//
// 背景底图使用**离线内嵌的粗粒度大陆/岛屿轮廓**（静态度量级，零网络依赖），
// 仅作视觉定位参考，不作为精确国界。

/// 简化大陆/岛屿轮廓（经纬度，粗粒度，用于地图背景定位）。
pub const CONTINENTS: &[&[(f64, f64)]] = &[
  // 北美洲
  &[
    (-168.0, 65.0),
    (-160.0, 70.0),
    (-140.0, 70.0),
    (-120.0, 72.0),
    (-100.0, 72.0),
    (-90.0, 68.0),
    (-85.0, 60.0),
    (-80.0, 55.0),
    (-70.0, 55.0),
    (-65.0, 47.0),
    (-70.0, 40.0),
    (-75.0, 35.0),
    (-80.0, 30.0),
    (-85.0, 25.0),
    (-90.0, 20.0),
    (-95.0, 16.0),
    (-90.0, 15.0),
    (-85.0, 10.0),
    (-83.0, 8.0),
    (-88.0, 13.0),
    (-92.0, 15.0),
    (-97.0, 16.0),
    (-105.0, 20.0),
    (-110.0, 24.0),
    (-115.0, 30.0),
    (-122.0, 37.0),
    (-125.0, 40.0),
    (-125.0, 48.0),
    (-130.0, 52.0),
    (-135.0, 57.0),
    (-145.0, 60.0),
    (-155.0, 60.0),
    (-165.0, 60.0),
    (-168.0, 65.0),
  ],
  // 南美洲
  &[
    (-80.0, 10.0),
    (-75.0, 12.0),
    (-70.0, 11.0),
    (-60.0, 8.0),
    (-52.0, 5.0),
    (-50.0, 0.0),
    (-45.0, -5.0),
    (-40.0, -10.0),
    (-38.0, -15.0),
    (-40.0, -22.0),
    (-48.0, -28.0),
    (-53.0, -34.0),
    (-58.0, -39.0),
    (-65.0, -47.0),
    (-68.0, -55.0),
    (-72.0, -55.0),
    (-75.0, -50.0),
    (-72.0, -42.0),
    (-70.0, -35.0),
    (-70.0, -28.0),
    (-75.0, -22.0),
    (-80.0, -15.0),
    (-80.0, -5.0),
    (-80.0, 5.0),
    (-80.0, 10.0),
  ],
  // 非洲
  &[
    (-17.0, 15.0),
    (-16.0, 21.0),
    (-15.0, 25.0),
    (-10.0, 30.0),
    (-5.0, 33.0),
    (0.0, 35.0),
    (10.0, 37.0),
    (20.0, 32.0),
    (30.0, 32.0),
    (33.0, 30.0),
    (35.0, 20.0),
    (40.0, 15.0),
    (50.0, 12.0),
    (45.0, 5.0),
    (40.0, -5.0),
    (35.0, -15.0),
    (30.0, -25.0),
    (25.0, -35.0),
    (20.0, -35.0),
    (15.0, -30.0),
    (10.0, -20.0),
    (5.0, -10.0),
    (0.0, 0.0),
    (-5.0, 5.0),
    (-10.0, 5.0),
    (-15.0, 10.0),
    (-17.0, 15.0),
  ],
  // 欧洲
  &[
    (-10.0, 45.0),
    (-5.0, 45.0),
    (0.0, 50.0),
    (5.0, 55.0),
    (10.0, 60.0),
    (15.0, 65.0),
    (20.0, 70.0),
    (30.0, 70.0),
    (40.0, 65.0),
    (45.0, 60.0),
    (40.0, 55.0),
    (35.0, 50.0),
    (30.0, 45.0),
    (25.0, 40.0),
    (20.0, 38.0),
    (15.0, 35.0),
    (10.0, 35.0),
    (5.0, 40.0),
    (0.0, 40.0),
    (-5.0, 40.0),
    (-10.0, 43.0),
    (-10.0, 45.0),
  ],
  // 亚洲
  &[
    (40.0, 65.0),
    (50.0, 70.0),
    (60.0, 70.0),
    (70.0, 70.0),
    (80.0, 72.0),
    (100.0, 75.0),
    (120.0, 72.0),
    (140.0, 70.0),
    (160.0, 65.0),
    (170.0, 65.0),
    (178.0, 60.0),
    (170.0, 55.0),
    (160.0, 50.0),
    (150.0, 45.0),
    (140.0, 40.0),
    (135.0, 35.0),
    (130.0, 30.0),
    (120.0, 25.0),
    (110.0, 20.0),
    (105.0, 10.0),
    (100.0, 5.0),
    (95.0, 10.0),
    (90.0, 20.0),
    (80.0, 25.0),
    (70.0, 25.0),
    (60.0, 25.0),
    (55.0, 25.0),
    (50.0, 30.0),
    (45.0, 30.0),
    (40.0, 35.0),
    (45.0, 40.0),
    (45.0, 50.0),
    (40.0, 55.0),
    (40.0, 65.0),
  ],
  // 澳大利亚
  &[
    (115.0, -20.0),
    (120.0, -15.0),
    (130.0, -12.0),
    (140.0, -15.0),
    (145.0, -18.0),
    (150.0, -20.0),
    (150.0, -25.0),
    (145.0, -30.0),
    (140.0, -35.0),
    (135.0, -35.0),
    (130.0, -32.0),
    (125.0, -30.0),
    (120.0, -25.0),
    (115.0, -25.0),
    (115.0, -20.0),
  ],
  // 格陵兰
  &[
    (-45.0, 60.0),
    (-40.0, 65.0),
    (-35.0, 70.0),
    (-30.0, 75.0),
    (-25.0, 80.0),
    (-20.0, 80.0),
    (-20.0, 75.0),
    (-25.0, 70.0),
    (-30.0, 65.0),
    (-40.0, 60.0),
    (-45.0, 60.0),
  ],
  // 南极洲
  &[
    (-180.0, -70.0),
    (-120.0, -72.0),
    (-60.0, -70.0),
    (0.0, -68.0),
    (60.0, -70.0),
    (120.0, -72.0),
    (180.0, -70.0),
    (180.0, -90.0),
    (-180.0, -90.0),
    (-180.0, -70.0),
  ],
  // 冰岛
  &[
    (-24.0, 66.0),
    (-18.0, 66.0),
    (-14.0, 64.0),
    (-18.0, 63.0),
    (-24.0, 63.0),
    (-24.0, 66.0),
  ],
  // 不列颠群岛
  &[
    (-6.0, 58.0),
    (-3.0, 53.0),
    (0.0, 51.0),
    (-4.0, 50.0),
    (-8.0, 51.0),
    (-8.0, 55.0),
    (-6.0, 58.0),
  ],
  // 日本列岛
  &[
    (141.0, 45.0),
    (144.0, 44.0),
    (143.0, 42.0),
    (141.0, 40.0),
    (140.0, 37.0),
    (138.0, 35.0),
    (135.0, 34.0),
    (132.0, 33.0),
    (130.0, 32.0),
    (131.0, 34.0),
    (134.0, 35.0),
    (137.0, 37.0),
    (139.0, 39.0),
    (141.0, 41.0),
    (141.0, 45.0),
  ],
  // 马达加斯加
  &[
    (44.0, -12.0),
    (47.0, -15.0),
    (50.0, -16.0),
    (50.0, -22.0),
    (47.0, -25.0),
    (44.0, -20.0),
    (44.0, -12.0),
  ],
  // 新西兰
  &[
    (173.0, -35.0),
    (176.0, -38.0),
    (178.0, -38.0),
    (178.0, -41.0),
    (175.0, -41.0),
    (172.0, -40.0),
    (170.0, -42.0),
    (167.0, -46.0),
    (168.0, -47.0),
    (171.0, -44.0),
    (173.0, -41.0),
    (173.0, -35.0),
  ],
  // 加勒比海（古巴 + 伊斯帕尼奥拉）
  &[
    (-84.0, 22.0),
    (-80.0, 23.0),
    (-77.0, 21.0),
    (-74.0, 20.0),
    (-72.0, 18.0),
    (-75.0, 17.0),
    (-79.0, 18.0),
    (-82.0, 21.0),
    (-84.0, 22.0),
  ],
  // 新几内亚
  &[
    (131.0, -1.0),
    (137.0, -2.0),
    (141.0, -3.0),
    (146.0, -6.0),
    (148.0, -9.0),
    (144.0, -9.0),
    (140.0, -8.0),
    (134.0, -4.0),
    (131.0, -1.0),
  ],
];

/// 主要 DXCC 实体名称 + 代表坐标（经度, 纬度），用于地图放大后的国家/地区标注。
/// 与 `ham_web_core::dxcc` 的 `DXCC_PREFIXES` 实体对齐，坐标取首都或地理中心。
pub const COUNTRY_LABELS: &[(&str, f64, f64)] = &[
  ("中国", 104.0, 35.0),
  ("台湾", 121.0, 23.7),
  ("香港", 114.1, 22.3),
  ("澳门", 113.5, 22.2),
  ("夏威夷", -157.0, 20.7),
  ("日本", 138.0, 36.0),
  ("韩国", 127.5, 36.5),
  ("印度", 79.0, 22.0),
  ("泰国", 100.5, 15.5),
  ("马来西亚", 102.0, 3.5),
  ("新加坡", 103.8, 1.3),
  ("印度尼西亚", 118.0, -2.5),
  ("菲律宾", 122.0, 13.0),
  ("英国", -2.0, 54.0),
  ("法国", 2.5, 46.5),
  ("德国", 10.0, 51.0),
  ("意大利", 12.5, 42.5),
  ("西班牙", -3.7, 40.4),
  ("葡萄牙", -8.0, 39.5),
  ("比利时", 4.6, 50.5),
  ("荷兰", 5.3, 52.1),
  ("瑞士", 8.0, 46.8),
  ("奥地利", 14.3, 47.5),
  ("芬兰", 26.0, 62.5),
  ("瑞典", 15.0, 60.0),
  ("挪威", 10.0, 60.5),
  ("丹麦", 10.0, 55.5),
  ("波兰", 19.0, 52.0),
  ("捷克", 14.5, 49.8),
  ("斯洛伐克", 19.5, 48.7),
  ("匈牙利", 19.5, 47.0),
  ("罗马尼亚", 25.0, 45.8),
  ("俄罗斯", 60.0, 58.0),
  ("乌克兰", 31.0, 49.0),
  ("美国", -98.0, 39.0),
  ("加拿大", -106.0, 56.0),
  ("墨西哥", -102.0, 23.5),
  ("阿根廷", -64.0, -34.0),
  ("巴西", -52.0, -10.0),
  ("智利", -70.0, -33.0),
  ("乌拉圭", -56.0, -32.5),
  ("澳大利亚", 133.0, -25.0),
  ("新西兰", 172.0, -42.0),
  ("南非", 24.0, -29.0),
  ("埃及", 30.0, 26.5),
  ("摩洛哥", -7.0, 31.8),
  ("以色列", 35.0, 31.3),
  ("沙特阿拉伯", 45.0, 24.0),
];

/// 等距圆柱投影：经纬度 → SVG 坐标。
pub fn project(lon: f64, lat: f64) -> (f64, f64) {
  ((lon + 180.0) * K, (90.0 - lat) * K)
}

/// 多边形 → SVG points 字符串。
pub fn polygon_points(poly: &[(f64, f64)]) -> String {
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
pub fn simplify(poly: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
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

/// 两个活动指针的欧氏距离（像素），用于双指捏合缩放。
fn ptr_dist(a: &(i32, f64, f64), b: &(i32, f64, f64)) -> f64 {
  let dx = a.1 - b.1;
  let dy = a.2 - b.2;
  (dx * dx + dy * dy).sqrt()
}

// ---------------------------------------------------------------------------
// ## 3. 颜色方案与视觉层次
// ---------------------------------------------------------------------------

/// field（20°×10°）热力：按密度分档的填充类。
fn field_fill_class(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.25 {
    "fill-primary/15"
  } else if r <= 0.5 {
    "fill-primary/25"
  } else if r <= 0.75 {
    "fill-primary/40"
  } else {
    "fill-primary/60"
  }
}

/// square（2°×1°）热力：更细粒度分档。
fn square_fill_class(n: usize, max: usize) -> &'static str {
  let r = n as f64 / max.max(1) as f64;
  if r <= 0.2 {
    "fill-primary/30"
  } else if r <= 0.4 {
    "fill-primary/50"
  } else if r <= 0.6 {
    "fill-primary/70"
  } else if r <= 0.8 {
    "fill-primary/85"
  } else {
    "fill-primary"
  }
}

/// 全局 square 索引 `(sl, sa)` → 4 位网格码字符串。
fn square_label((sl, sa): (usize, usize)) -> String {
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

/// Maidenhead 网格地图：等距圆柱投影，square（2°×1°）精细热力 + 大陆轮廓 + 缩放平移 + field 点击展开。
///
/// - `entries`：全部通联记录（含呼号 / 频率 / 模式 / 网格），用于聚合与明细展示。
/// - `station_grid`：本台网格（MY_GRIDSQUARE），非空时在地图上标记“家”位置。
#[component]
pub fn GridMap(entries: Vec<LogEntry>, station_grid: String) -> impl IntoView {
  let entries = Arc::new(entries);

  // ---- 筛选状态：波段 / 模式（独立、可叠加，AND 关系）----
  let band_filter = RwSignal::new(None::<String>);
  let mode_filter = RwSignal::new(None::<String>);

  // 可筛选的波段 / 模式列表（一次性，按出现情况）。
  let mut band_set: HashSet<String> = HashSet::new();
  let mut mode_list: Vec<String> = Vec::new();
  for e in entries.iter() {
    if let Ok(f) = e.freq.trim().parse::<f64>() {
      let b = band_of(f);
      if b != "其他" {
        band_set.insert(b.to_owned());
      }
    }
    if !mode_list.contains(&e.mode) {
      mode_list.push(e.mode.clone());
    }
  }
  let mut bands: Vec<String> = Vec::new();
  for &b in BAND_ORDER {
    if band_set.contains(b) {
      bands.push(b.to_string());
    }
  }
  mode_list.sort();

  let matches_filter = move |e: &LogEntry| {
    if let Some(b) = band_filter.get() {
      let ok = e
        .freq
        .trim()
        .parse::<f64>()
        .map(|f| band_of(f) == b.as_str())
        .unwrap_or(false);
      if !ok {
        return false;
      }
    }
    if let Some(m) = mode_filter.get()
      && e.mode != m
    {
      return false;
    }
    true
  };

  // 筛选后的记录
  let filtered = Memo::new(move |_| {
    entries
      .iter()
      .filter(|e| matches_filter(e))
      .cloned()
      .collect::<Vec<LogEntry>>()
  });

  // field 聚合：field 索引 → 去重网格码（用于展开列表与标签）。
  let field_grids = Memo::new(move |_| {
    let mut m: HashMap<(usize, usize), Vec<String>> = HashMap::new();
    for e in filtered.get() {
      if let Some(idx) = field_index(&e.gridsquare) {
        let list = m.entry(idx).or_default();
        if !list.contains(&e.gridsquare) {
          list.push(e.gridsquare.clone());
        }
      }
    }
    m
  });

  // square 聚合：square 索引 → 完整记录列表（用于热力计数与明细展示）。
  let square_entries = Memo::new(move |_| {
    let mut m: HashMap<(usize, usize), Vec<LogEntry>> = HashMap::new();
    for e in filtered.get() {
      if let Some(idx) = square_index(&e.gridsquare) {
        m.entry(idx).or_default().push(e.clone());
      }
    }
    m
  });

  let field_max = Memo::new(move |_| {
    field_grids
      .get()
      .values()
      .map(|v| v.len())
      .max()
      .unwrap_or(1)
  });
  let square_max = Memo::new(move |_| {
    square_entries
      .get()
      .values()
      .map(|v| v.len())
      .max()
      .unwrap_or(1)
  });
  let grid_count = Memo::new(move |_| {
    filtered
      .get()
      .iter()
      .map(|e| e.gridsquare.clone())
      .collect::<HashSet<_>>()
      .len()
  });
  let dxcc_count = Memo::new(move |_| {
    filtered
      .get()
      .iter()
      .filter_map(|e| dxcc_entity(&e.callsign).map(|s| s.to_owned()))
      .collect::<HashSet<_>>()
      .len()
  });

  // ---- 选中状态：field 展开 / square 明细 / 搜索高亮 ----
  let selected = RwSignal::new(None::<(usize, usize)>);
  let selected_square = RwSignal::new(None::<(usize, usize)>);
  let highlight = RwSignal::new(None::<String>);
  let search_input = RwSignal::new(String::new());
  let show_grayline = RwSignal::new(false);

  // ---- 灰线实时时钟（60s 刷新，与灰线页一致）----
  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  set_interval(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  );

  // ---- §7 交互：视图状态（viewBox 平移/缩放，x 允许环绕）----
  let view_box = RwSignal::new((0.0f64, 0.0f64, MAP_W, MAP_H));
  let zoom = Signal::derive(move || MAP_W / view_box.get().2);
  let dragging = RwSignal::new(false);
  let drag_start = StoredValue::new((0.0f64, 0.0f64, 0.0f64, 0.0f64));
  let svg_ref = NodeRef::<svg::Svg>::new();
  let active_pointers = RwSignal::new(Vec::<(i32, f64, f64)>::new());
  let pinch_state = StoredValue::new((0.0f64, (0.0f64, 0.0f64, MAP_W, MAP_H)));
  let hover = RwSignal::new(None::<(f64, f64, f64, f64)>);

  let zoom_by = move |factor: f64| {
    let (x, y, w, h) = view_box.get();
    let nw = (w * factor).clamp(MAP_W / 16.0, MAP_W);
    let nh = (h * factor).clamp(MAP_H / 16.0, MAP_H);
    let nx = (x + (w - nw) / 2.0).rem_euclid(MAP_W);
    let ny = (y + (h - nh) / 2.0).clamp(0.0, MAP_H - nh);
    view_box.set((nx, ny, nw, nh));
  };
  let zoom_in = move |_| zoom_by(0.7);
  let zoom_out = move |_| zoom_by(1.0 / 0.7);
  let reset_view = move |_| view_box.set((0.0, 0.0, MAP_W, MAP_H));

  // 搜索定位：解析网格 → 缩放到 square 级别并居中 → 红色十字高亮。
  let do_search = move |_| {
    let g = search_input.get().trim().to_uppercase();
    if let Some((lat, lon)) = lat_lon_from_grid(&g) {
      let (px, py) = project(lon, lat);
      let w = MAP_W / 8.0;
      let h = MAP_H / 8.0;
      view_box.set((
        (px - w / 2.0).rem_euclid(MAP_W),
        (py - h / 2.0).clamp(0.0, MAP_H - h),
        w,
        h,
      ));
      highlight.set(Some(g));
    } else {
      highlight.set(None);
    }
  };

  let on_wheel = move |ev: web_sys::WheelEvent| {
    ev.prevent_default();
    let Some(el) = svg_ref.get() else {
      return;
    };
    let rect = el.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
      return;
    }
    let factor = if ev.delta_y() < 0.0 { 0.8 } else { 1.25 };
    let px = (f64::from(ev.client_x()) - rect.left()) / rect.width();
    let py = (f64::from(ev.client_y()) - rect.top()) / rect.height();
    let (x, y, w, h) = view_box.get();
    let nw = (w * factor).clamp(MAP_W / 16.0, MAP_W);
    let nh = (h * factor).clamp(MAP_H / 16.0, MAP_H);
    let wx = x + px * w;
    let wy = y + py * h;
    let nx = (wx - px * nw).rem_euclid(MAP_W);
    let ny = (wy - py * nh).clamp(0.0, MAP_H - nh);
    view_box.set((nx, ny, nw, nh));
  };

  let on_pointer_down = move |ev: web_sys::PointerEvent| {
    if ev.button() != 0 {
      return;
    }
    if let Some(el) = svg_ref.get() {
      let _ = el.set_pointer_capture(ev.pointer_id());
    }
    let mut pts = active_pointers.get_untracked();
    pts.retain(|p| p.0 != ev.pointer_id());
    pts.push((
      ev.pointer_id(),
      f64::from(ev.client_x()),
      f64::from(ev.client_y()),
    ));
    if pts.len() == 1 {
      dragging.set(true);
      let (x, y, _, _) = view_box.get_untracked();
      drag_start.set_value((f64::from(ev.client_x()), f64::from(ev.client_y()), x, y));
    } else if pts.len() == 2 {
      dragging.set(false);
      let dist = ptr_dist(&pts[0], &pts[1]);
      pinch_state.set_value((dist, view_box.get_untracked()));
    }
    active_pointers.set(pts);
  };
  let on_pointer_move = move |ev: web_sys::PointerEvent| {
    let mut pts = active_pointers.get_untracked();
    let mut present = false;
    for p in pts.iter_mut() {
      if p.0 == ev.pointer_id() {
        p.1 = f64::from(ev.client_x());
        p.2 = f64::from(ev.client_y());
        present = true;
      }
    }
    if !present {
      return;
    }
    active_pointers.set(pts.clone());

    if pts.len() == 1 {
      if !dragging.get_untracked() {
        return;
      }
      let Some(el) = svg_ref.get() else {
        return;
      };
      let rect = el.get_bounding_client_rect();
      if rect.width() <= 0.0 {
        return;
      }
      let (sx, sy, sx0, sy0) = drag_start.get_value();
      let (_, _, w, h) = view_box.get_untracked();
      let dx = (pts[0].1 - sx) * (w / rect.width());
      let dy = (pts[0].2 - sy) * (h / rect.height());
      view_box.set((
        (sx0 - dx).rem_euclid(MAP_W),
        (sy0 - dy).clamp(0.0, MAP_H - h),
        w,
        h,
      ));
    } else if pts.len() >= 2 {
      let (start_dist, start_box) = pinch_state.get_value();
      let cur = ptr_dist(&pts[0], &pts[1]);
      if start_dist > 0.0 && cur > 0.0 {
        let factor = start_dist / cur;
        let (x, y, w, h) = start_box;
        let nw = (w * factor).clamp(MAP_W / 16.0, MAP_W);
        let nh = (h * factor).clamp(MAP_H / 16.0, MAP_H);
        let nx = (x + (w - nw) / 2.0).rem_euclid(MAP_W);
        let ny = (y + (h - nh) / 2.0).clamp(0.0, MAP_H - nh);
        view_box.set((nx, ny, nw, nh));
      }
    }
  };
  let on_pointer_up = move |ev: web_sys::PointerEvent| {
    let mut pts = active_pointers.get_untracked();
    pts.retain(|p| p.0 != ev.pointer_id());
    if pts.len() == 1 {
      dragging.set(true);
      let (x, y, _, _) = view_box.get_untracked();
      drag_start.set_value((pts[0].1, pts[0].2, x, y));
    } else if pts.is_empty() {
      dragging.set(false);
    }
    active_pointers.set(pts);
  };

  let on_mouse_move = move |ev: web_sys::MouseEvent| {
    if dragging.get_untracked() || !active_pointers.get_untracked().is_empty() {
      hover.set(None);
      return;
    }
    let Some(el) = svg_ref.get() else {
      return;
    };
    let rect = el.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
      return;
    }
    let (vx, vy, vw, vh) = view_box.get_untracked();
    let css_x = f64::from(ev.client_x()) - rect.left();
    let css_y = f64::from(ev.client_y()) - rect.top();
    let svg_x = vx + css_x / rect.width() * vw;
    let svg_y = vy + css_y / rect.height() * vh;
    // 经度环绕归一（viewBox.x 可能越过反子午线）。
    let lon = (svg_x / K).rem_euclid(360.0) - 180.0;
    let lat = 90.0 - svg_y / K;
    hover.set(Some((css_x, css_y, lon, lat)));
  };
  let on_mouse_leave = move |_: web_sys::MouseEvent| hover.set(None);

  // ---- 静态图层（不依赖响应式状态，构建一次后随环绕平铺克隆）----
  let land_views = CONTINENTS
    .iter()
    .map(|poly| {
      let pts = polygon_points(&simplify(poly, 0.25));
      view! {
        <polygon
          points=pts
          class="fill-foreground/10 stroke-foreground/20"
          stroke-width="0.5"
          vector-effect="non-scaling-stroke"
        />
      }
    })
    .collect_view();

  let graticule_views = (0..=18)
    .map(|i| {
      let x = (i * 40).to_string();
      view! {
        <line
          x1=x.clone()
          y1="0"
          x2=x
          y2="360"
          class="stroke-foreground/10"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
      }
    })
    .collect_view();
  let graticule_h_views = (0..=18)
    .map(|i| {
      let y = (i * 20).to_string();
      view! {
        <line
          x1="0"
          y1=y.clone()
          x2="720"
          y2=y
          class="stroke-foreground/10"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
      }
    })
    .collect_view();

  let tick_views = (0..=12)
    .map(|k| {
      let lon = -180.0 + k as f64 * 30.0;
      let x = ((lon + 180.0) * K).to_string();
      let dir = if lon < 0.0 { "W" } else if lon > 0.0 { "E" } else { "" };
      view! {
        <text x=x y="356" text-anchor="middle" class="fill-muted-foreground/70 font-mono" font-size="8">
          {format!("{:.0}°{dir}", lon.abs())}
        </text>
      }
    })
    .collect_view();
  let tick_lat_views = (0..=6)
    .map(|k| {
      let lat = -90.0 + k as f64 * 30.0;
      let y = ((90.0 - lat) * K).to_string();
      let dir = if lat < 0.0 { "S" } else if lat > 0.0 { "N" } else { "" };
      view! {
        <text x="4" y=y text-anchor="start" class="fill-muted-foreground/70 font-mono" font-size="8">
          {format!("{:.0}°{dir}", lat.abs())}
        </text>
      }
    })
    .collect_view();

  let name_views = [
    ("亚洲", 95.0, 48.0, "fill-foreground/40", "11", false),
    ("欧洲", 25.0, 55.0, "fill-foreground/40", "11", false),
    ("非洲", 20.0, 2.0, "fill-foreground/40", "11", false),
    ("北美洲", -100.0, 48.0, "fill-foreground/40", "11", false),
    ("南美洲", -58.0, -12.0, "fill-foreground/40", "11", false),
    ("大洋洲", 133.0, -25.0, "fill-foreground/40", "11", false),
    ("南极洲", 0.0, -83.0, "fill-foreground/40", "11", false),
    ("太平洋", -152.0, 5.0, "fill-foreground/25", "10", true),
    ("大西洋", -28.0, 22.0, "fill-foreground/25", "10", true),
    ("印度洋", 72.0, -22.0, "fill-foreground/25", "10", true),
  ]
  .into_iter()
  .map(|(name, lon, lat, fill, size, italic)| {
    let (x, y) = project(lon, lat);
    let cls = if italic { format!("{fill} italic") } else { fill.to_owned() };
    view! {
      <text x=x.to_string() y=y.to_string() text-anchor="middle" class=cls font-size=size>{name}</text>
    }
  })
  .collect_view();

  let country_views = COUNTRY_LABELS
    .iter()
    .map(|&(name, lon, lat)| {
      let (x, y) = project(lon, lat);
      view! {
        <text x=x.to_string() y=y.to_string() text-anchor="middle" class="fill-foreground/40" font-size="8">{name}</text>
      }
    })
    .collect_view();

  // 本台“家”标记（静态，非空时构建）。
  let home_marker = {
    let mark: Option<(String, String, String)> =
      lat_lon_from_grid(&station_grid).map(|(lat, lon)| {
        let (x, y) = project(lon, lat);
        (
          x.to_string(),
          y.to_string(),
          format!("本台网格 {station_grid}"),
        )
      });
    view! {
      <g>
        {mark.map(|(cx, cy, label)| {
          view! {
            <g class="stroke-primary" vector-effect="non-scaling-stroke">
              <circle cx=cx.clone() cy=cy.clone() r="6" fill="none" stroke-width="2" />
              <circle cx=cx cy=cy r="2" class="fill-primary" />
              <title>{label}</title>
            </g>
          }
        })}
      </g>
    }
  };

  view! {
    <div>
      // 工具栏：搜索定位 + 波段 / 模式筛选 + 灰线开关
      <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
        <div class="flex items-center gap-1">
          <input
            type="text"
            placeholder="定位网格，如 OM89EW"
            class=input_class("h-8 w-36 font-mono uppercase")
            prop:value=move || search_input.get()
            on:input=move |e| search_input.set(event_target_value(&e).to_uppercase())
          />
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=do_search>
            "定位"
          </button>
        </div>
        <select
          class=input_class("h-8 w-auto")
          on:change=move |e| {
            let v = event_target_value(&e);
            band_filter.set(if v.is_empty() { None } else { Some(v) });
          }
        >
          <option value="">"全部波段"</option>
          {bands
            .iter()
            .map(|b| {
              view! { <option value=b.clone()>{b.clone()}</option> }
            })
            .collect_view()}
        </select>
        <select
          class=input_class("h-8 w-auto")
          on:change=move |e| {
            let v = event_target_value(&e);
            mode_filter.set(if v.is_empty() { None } else { Some(v) });
          }
        >
          <option value="">"全部模式"</option>
          {mode_list
            .iter()
            .map(|m| {
              view! { <option value=m.clone()>{m.clone()}</option> }
            })
            .collect_view()}
        </select>
        <label class="flex cursor-pointer items-center gap-1.5 text-muted-foreground">
          <input
            type="checkbox"
            class="size-4 accent-primary"
            prop:checked=move || show_grayline.get()
            on:change=move |e| show_grayline.set(event_target_checked(&e))
          />
          "灰线"
        </label>
      </div>

      // 统计概览徽章
      <div class="mb-2 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
        <span class="rounded-full border bg-muted/40 px-2 py-0.5">
          "field " <span class="font-semibold tabular-nums">{move || field_grids.get().len()}</span>
        </span>
        <span class="rounded-full border bg-muted/40 px-2 py-0.5">
          "square " <span class="font-semibold tabular-nums">{move || square_entries.get().len()}</span>
        </span>
        <span class="rounded-full border bg-muted/40 px-2 py-0.5">
          "网格 " <span class="font-semibold tabular-nums">{move || grid_count.get()}</span>
        </span>
        <span class="rounded-full border bg-muted/40 px-2 py-0.5">
          "DXCC " <span class="font-semibold tabular-nums">{move || dxcc_count.get()}</span>
        </span>
      </div>

      <div class="relative">
        <svg
          node_ref=svg_ref
          viewBox=move || {
            let (x, y, w, h) = view_box.get();
            format!("{x:.2} {y:.2} {w:.2} {h:.2}")
          }
          class="block w-full select-none"
          style=move || {
            if dragging.get() { "touch-action: none; cursor: grabbing;" } else { "touch-action: none; cursor: grab;" }
          }
          role="img"
          aria-label="已通联网格地图（滚轮缩放、拖拽平移、双指缩放、双击复位、反子午线环绕）"
          preserveAspectRatio="xMidYMid meet"
          on:wheel=on_wheel
          on:pointerdown=on_pointer_down
          on:pointermove=on_pointer_move
          on:pointerup=on_pointer_up
          on:pointerleave=on_pointer_up
          on:pointercancel=on_pointer_up
          on:mousemove=on_mouse_move
          on:mouseleave=on_mouse_leave
          on:dblclick=reset_view
        >
          <defs>
            <linearGradient id="ocean-grad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color="var(--muted)" stop-opacity="0.5" />
              <stop offset="50%" stop-color="var(--muted)" stop-opacity="0.28" />
              <stop offset="100%" stop-color="var(--muted)" stop-opacity="0.45" />
            </linearGradient>
            // 太阳直射点光晕（灰线叠加用）
            <radialGradient id="sun-glow">
              <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.8" />
              <stop offset="100%" stop-color="#f59e0b" stop-opacity="0" />
            </radialGradient>
          </defs>
          // 世界内容在 x 方向平铺三份，实现反子午线无缝环绕。
          {WORLD_OFFSETS
            .into_iter()
            .map(|k| {
              let t = format!("translate({} 0)", k * MAP_W);
              view! {
                <g transform=t>
                  // L0 海洋
                  <rect x="0" y="0" width="720" height="360" fill="url(#ocean-grad)" />
                  // L1 陆地
                  <g shape-rendering="geometricPrecision">{land_views.clone()}</g>
                  // L2 经纬网
                  {graticule_views.clone()}
                  {graticule_h_views.clone()}
                  <line x1="0" y1="180" x2="720" y2="180" class="stroke-foreground/25" stroke-width="1" vector-effect="non-scaling-stroke" />
                  <line x1="360" y1="0" x2="360" y2="360" class="stroke-foreground/25" stroke-width="1" vector-effect="non-scaling-stroke" />
                  // 刻度与名称标注
                  {tick_views.clone()}
                  {tick_lat_views.clone()}
                  {name_views.clone()}
                  // 国家/地区名称（放大后显示）
                  <g
                    class="fill-foreground/40"
                    style=move || {
                      if zoom.get() >= 2.0 { "opacity:0.9" } else { "opacity:0" }
                    }
                  >
                    {country_views.clone()}
                  </g>
                  // L3 field 热力
                  {move || {
                    let fmax = field_max.get();
                    field_grids
                      .get()
                      .iter()
                      .map(|((fl, fa), list)| {
                        let x = (fl * 40).to_string();
                        let y = ((17 - fa) * 20).to_string();
                        view! {
                          <rect x=x y=y width="40" height="20" class=field_fill_class(list.len(), fmax) />
                        }
                      })
                      .collect_view()
                  }}
                  // L4 square 热力（含悬停 tooltip）
                  {move || {
                    let smax = square_max.get();
                    square_entries
                      .get()
                      .iter()
                      .map(|((sl, sa), list)| {
                        let x = (sl * 4).to_string();
                        let y = ((179 - sa) * 2).to_string();
                        let label = square_label((*sl, *sa));
                        let n = list.len();
                        view! {
                          <rect x=x y=y width="4" height="2" class=square_fill_class(n, smax)>
                            <title>{format!("{label}：{n} 条通联")}</title>
                          </rect>
                        }
                      })
                      .collect_view()
                  }}
                  // field 标签（缩放级别控制显隐）
                  <g
                    class="fill-muted-foreground"
                    style=move || {
                      if zoom.get() >= 0.6 { "opacity:0.75" } else { "opacity:0" }
                    }
                  >
                    {move || {
                      field_grids
                        .get()
                        .iter()
                        .map(|((fl, fa), _)| {
                          let cx = (fl * 40 + 20).to_string();
                          let cy = ((17 - fa) * 20 + 10).to_string();
                          let f_lon = (b'A' + *fl as u8) as char;
                          let f_lat = (b'A' + *fa as u8) as char;
                          view! {
                            <text x=cx y=cy text-anchor="middle" dominant-baseline="middle" class="fill-muted-foreground" font-size="7">
                              {format!("{f_lon}{f_lat}")}
                            </text>
                          }
                        })
                        .collect_view()
                    }}
                  </g>
                  // 选中 field 轮廓
                  {move || {
                    selected.get().map(|(fl, fa)| {
                      let x = (fl * 40).to_string();
                      let y = ((17 - fa) * 20).to_string();
                      view! {
                        <rect
                          x=x
                          y=y
                          width="40"
                          height="20"
                          fill="none"
                          class="stroke-ring"
                          stroke-width="2.5"
                          vector-effect="non-scaling-stroke"
                        />
                      }
                    })
                  }}
                  // 选中 square 轮廓
                  {move || {
                    selected_square.get().map(|(sl, sa)| {
                      let x = (sl * 4).to_string();
                      let y = ((179 - sa) * 2).to_string();
                      view! {
                        <rect
                          x=x
                          y=y
                          width="4"
                          height="2"
                          fill="none"
                          class="stroke-ring"
                          stroke-width="2"
                          vector-effect="non-scaling-stroke"
                        />
                      }
                    })
                  }}
                  // 本台“家”标记
                  {home_marker.clone()}
                  // 灰线叠加（晨昏圈 + 夜半球遮罩）
                  {move || {
                    if !show_grayline.get() {
                      return ().into_any();
                    }
                    let ms = now.get();
                    let date = js_sys::Date::new(&JsValue::from_f64(ms));
                    let y = date.get_utc_full_year();
                    let mo = date.get_utc_month() + 1;
                    let d = date.get_utc_date();
                    let doy = day_of_year(y, mo, d);
                    let hour = date.get_utc_hours() as f64 + date.get_utc_minutes() as f64 / 60.0;
                    let decl = solar_declination(doy);
                    let sslon = subsolar_longitude(hour);
                    // 太阳直射点（正午）与反日点（午夜）
                    let (sx, sy) = project(sslon, decl);
                    let (ax, ay) = project(
                      if sslon >= 0.0 { sslon - 180.0 } else { sslon + 180.0 },
                      -decl,
                    );
                    view! {
                      <g>
                        <path d=night_path(decl, sslon) fill="#020617" opacity="0.22" />
                        <path d=band_path(decl, sslon, 6.0) fill="#f59e0b" opacity="0.14" />
                        <path d=band_path(decl, sslon, 2.0) fill="#f59e0b" opacity="0.28" />
                        <path
                          d=band_path(decl, sslon, 0.3)
                          fill="none"
                          stroke="#f59e0b"
                          stroke-width="1"
                          opacity="0.7"
                          vector-effect="non-scaling-stroke"
                        />
                        // 太阳直射点：光晕 + 实心
                        <circle cx=sx.to_string() cy=sy.to_string() r="16" fill="url(#sun-glow)" />
                        <circle cx=sx.to_string() cy=sy.to_string() r="4" fill="#f59e0b" opacity="0.9" />
                        // 反日点（午夜）
                        <circle
                          cx=ax.to_string()
                          cy=ay.to_string()
                          r="3"
                          fill="none"
                          stroke="#f59e0b"
                          stroke-width="1"
                          opacity="0.5"
                          vector-effect="non-scaling-stroke"
                        />
                      </g>
                    }
                    .into_any()
                  }}
                  // field 交互层（点击展开）
                  {move || {
                    field_grids
                      .get()
                      .iter()
                      .map(|((fl, fa), list)| {
                        let x = (fl * 40).to_string();
                        let y = ((17 - fa) * 20).to_string();
                        let f_lon = (b'A' + *fl as u8) as char;
                        let f_lat = (b'A' + *fa as u8) as char;
                        let key = (*fl, *fa);
                        let n = list.len();
                        view! {
                          <rect
                            x=x
                            y=y
                            width="40"
                            height="20"
                            fill="transparent"
                            class="cursor-pointer"
                            on:click=move |_| {
                              selected.set(Some(key));
                              selected_square.set(None);
                            }
                          >
                            <title>{format!("{f_lon}{f_lat}：{n} 个网格，点击查看")}</title>
                          </rect>
                        }
                      })
                      .collect_view()
                  }}
                  // square 交互层（点击展开明细，覆盖在 field 交互层之上）
                  {move || {
                    square_entries
                      .get()
                      .iter()
                      .map(|((sl, sa), list)| {
                        let x = (sl * 4).to_string();
                        let y = ((179 - sa) * 2).to_string();
                        let key = (*sl, *sa);
                        let label = square_label(key);
                        let n = list.len();
                        view! {
                          <rect
                            x=x
                            y=y
                            width="4"
                            height="2"
                            fill="transparent"
                            class="cursor-pointer"
                            on:click=move |_| {
                              selected_square.set(Some(key));
                              selected.set(None);
                            }
                          >
                            <title>{format!("{label}：{n} 条，点击查看明细")}</title>
                          </rect>
                        }
                      })
                      .collect_view()
                  }}
                  // 查询高亮标记
                  {move || {
                    highlight.get().map(|g| {
                      let (lat, lon) = lat_lon_from_grid(&g).unwrap_or((0.0, 0.0));
                      let (x, y) = project(lon, lat);
                      view! {
                        <g class="stroke-red-500" vector-effect="non-scaling-stroke">
                          <circle cx=x.to_string() cy=y.to_string() r="9" fill="none" stroke-width="2.5" />
                          <line x1=(x - 13.0).to_string() y1=y.to_string() x2=(x + 13.0).to_string() y2=y.to_string() stroke-width="2" />
                          <line x1=x.to_string() y1=(y - 13.0).to_string() x2=x.to_string() y2=(y + 13.0).to_string() stroke-width="2" />
                        </g>
                      }
                    })
                  }}
                </g>
              }
            })
            .collect_view()}
        </svg>

        {move || {
          hover.get().map(|(css_x, css_y, lon, lat)| {
            let lat_dir = if lat < 0.0 { "S" } else { "N" };
            let lon_dir = if lon < 0.0 { "W" } else { "E" };
            view! {
              <div
                class="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-full rounded bg-foreground/90 px-1.5 py-0.5 font-mono text-[10px] leading-none text-background shadow"
                style=format!("left: {:.0}px; top: {:.0}px;", css_x, css_y)
              >
                {format!("{:.1}°{}  {:.1}°{}", lat.abs(), lat_dir, lon.abs(), lon_dir)}
              </div>
            }
          })
        }}

        <div class="absolute right-2 top-2 flex flex-col gap-1">
          <button
            type="button"
            class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
            on:click=zoom_in
            aria-label="放大"
          >
            "+"
          </button>
          <button
            type="button"
            class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
            on:click=zoom_out
            aria-label="缩小"
          >
            "−"
          </button>
          <button
            type="button"
            class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
            on:click=reset_view
            aria-label="复位视图"
          >
            "⤢"
          </button>
        </div>
      </div>

      // 图例 + 交互提示
      <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        <span class="flex items-center gap-1.5">
          <span class="text-muted-foreground">"密度"</span>
          <span class="inline-block h-2 w-24 rounded-full bg-gradient-to-r from-primary/15 to-primary"></span>
          <span>"低 → 高"</span>
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-3 w-3 rounded-full border-2 border-primary"></span>
          <span>"本台"</span>
        </span>
        {move || {
          show_grayline.get().then(|| {
            view! {
              <span class="flex items-center gap-1.5">
                <span class="inline-block h-3 w-3 rounded-sm bg-amber-500/40"></span>
                <span>"晨昏圈"</span>
                <span class="ml-1 inline-block h-2.5 w-2.5 rounded-full bg-amber-500"></span>
                <span>"太阳直射点"</span>
                <span class="inline-block h-2.5 w-2.5 rounded-full border border-amber-500"></span>
                <span>"反日点"</span>
              </span>
            }
          })
        }}
        <span class="text-muted-foreground">"滚轮/双指缩放 · 拖拽平移 · 双击复位 · 输入网格定位 · 点击 field/square 展开"</span>
      </div>

      // 选中 field 的展开列表
      {move || {
        selected.get().map(|(fl, fa)| {
          let f_lon = (b'A' + fl as u8) as char;
          let f_lat = (b'A' + fa as u8) as char;
          let list = field_grids.get().get(&(fl, fa)).cloned().unwrap_or_default();
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3">
              <div class="mb-2 flex items-center justify-between text-xs">
                <span class="font-semibold">"网格 " {f_lon}{f_lat}</span>
                <button
                  type="button"
                  class="text-muted-foreground transition-colors hover:text-foreground"
                  on:click=move |_| selected.set(None)
                >
                  "关闭"
                </button>
              </div>
              <div class="flex flex-wrap gap-1.5">
                {list
                  .into_iter()
                  .map(|g| {
                    view! {
                      <span class="rounded bg-muted/60 px-2 py-0.5 font-mono text-xs">{g}</span>
                    }
                  })
                  .collect_view()}
              </div>
            </div>
          }
        })
      }}

      // 选中 square 的通联明细
      {move || {
        selected_square.get().map(|(sl, sa)| {
          let label = square_label((sl, sa));
          let list = square_entries.get().get(&(sl, sa)).cloned().unwrap_or_default();
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3">
              <div class="mb-2 flex items-center justify-between text-xs">
                <span class="font-semibold">"网格 " {label} " · " {list.len()} " 条"</span>
                <button
                  type="button"
                  class="text-muted-foreground transition-colors hover:text-foreground"
                  on:click=move |_| selected_square.set(None)
                >
                  "关闭"
                </button>
              </div>
              <div class="flex flex-wrap gap-1.5">
                {list
                  .iter()
                  .map(|e| {
                    let band = e.freq.trim().parse::<f64>().map(band_of).unwrap_or("—");
                    view! {
                      <span class="rounded bg-muted/60 px-2 py-0.5 font-mono text-xs">
                        {format!("{} · {} · {} · {}", e.callsign, band, e.mode, e.date)}
                      </span>
                    }
                  })
                  .collect_view()}
              </div>
            </div>
          }
        })
      }}
    </div>
  }
}
