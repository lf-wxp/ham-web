//! 网格地图（Maidenhead Grid Locator 世界地图）。
//!
//! 底图绘制与缩放/平移/反子午线环绕等交互统一由 [`crate::pages::map::MapView`] 提供；
//! 本模块专注网格业务：通联记录的 field / square 聚合、波段 / 模式筛选、灰线叠加开关、
//! 搜索定位与 field / square 点击展开。投影方式与交互决策见 [`crate::pages::map`] 模块文档。

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::grid::{
  distance_bearing, field_index, grid_from_lat_lon, lat_lon_from_grid, square_index,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::pages::map::{GraylineOverlay, MapView, project};
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::copy_text;

use crate::pages::log::LogEntry;

use super::grid_fill::{
  band_color, field_fill_class, square_fill_class, square_fill_class_confirmed,
  square_fill_class_sent,
};
use super::grid_filter::{GridFilters, QslStatus};
use super::grid_geo::{great_circle_d, square_center, square_label};

/// 波段显示顺序（低频 → 高频），筛选下拉据此排序。
const BAND_ORDER: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

/// 缩放阈值：`zoom` 达到该值后，地图热力由 field（20°×10°）切换为 square（2°×1°）。
const FIELD_SQUARE_ZOOM: f64 = 4.0;

/// 全球 square 总数（180 经度 × 180 纬度 = 2°×1° 网格）。
const WORLD_SQUARES: usize = 180 * 180;

/// 通联路径最多绘制条数（按网格 + 波段去重后），避免大日志拖慢渲染。
const MAX_PATHS: usize = 600;

/// Maidenhead 网格地图：square（2°×1°）精细热力 + 大陆轮廓 + 缩放平移 + field 点击展开。
///
/// - `entries`：全部通联记录（含呼号 / 频率 / 模式 / 网格），用于聚合与明细展示。
/// - `station_grid`：本台网格（MY_GRIDSQUARE），非空时在地图上标记“家”位置。
#[component]
pub fn GridMap(entries: Vec<LogEntry>, station_grid: String) -> impl IntoView {
  let entries = Arc::new(entries);
  // 是否无通联记录（用于空状态提示）。
  let entries_empty = entries.is_empty();

  // ---- 筛选状态：波段 / 模式 / DXCC / 年份 / 呼号（独立、可叠加，AND 关系）----
  // 从 localStorage 恢复上次筛选，默认全空。
  let saved_filters: GridFilters =
    crate::util::storage::get_json("grid-map-filters").unwrap_or_default();
  let band_filter = RwSignal::new(saved_filters.band.clone());
  let mode_filter = RwSignal::new(saved_filters.mode.clone());
  let dxcc_filter = RwSignal::new(saved_filters.dxcc.clone());
  let year_filter = RwSignal::new(saved_filters.year.clone());
  let callsign_query = RwSignal::new(saved_filters.callsign.clone());
  let qsl_filter = RwSignal::new(saved_filters.qsl);
  let show_paths = RwSignal::new(saved_filters.paths);

  // 可筛选的波段 / 模式 / DXCC / 年份列表（一次性，按出现情况）。
  let mut band_set: HashSet<String> = HashSet::new();
  let mut mode_list: Vec<String> = Vec::new();
  let mut dxcc_set: HashSet<String> = HashSet::new();
  let mut year_list: Vec<String> = Vec::new();
  for e in entries.iter() {
    let b = e.band_label();
    if !b.is_empty() {
      band_set.insert(b);
    }
    if !mode_list.contains(&e.mode) {
      mode_list.push(e.mode.clone());
    }
    if let Some(entity) = e.entity().map(|x| x.name) {
      dxcc_set.insert(entity.to_owned());
    }
    if let Some(y) = e.date.get(0..4)
      && !year_list.iter().any(|s| s == y)
    {
      year_list.push(y.to_owned());
    }
  }
  let mut bands: Vec<String> = Vec::new();
  for &b in BAND_ORDER {
    if band_set.contains(b) {
      bands.push(b.to_string());
    }
  }
  mode_list.sort();
  let mut dxcc_list: Vec<String> = dxcc_set.into_iter().collect();
  dxcc_list.sort();
  year_list.sort();

  let matches_filter = move |e: &LogEntry| {
    if let Some(b) = band_filter.get()
      && e.band_label() != b
    {
      return false;
    }
    if let Some(m) = mode_filter.get()
      && e.mode != m
    {
      return false;
    }
    if let Some(d) = dxcc_filter.get()
      && e.entity().map(|x| x.name) != Some(d.as_str())
    {
      return false;
    }
    if let Some(y) = year_filter.get()
      && e.date.get(0..4) != Some(y.as_str())
    {
      return false;
    }
    let q = callsign_query.get().trim().to_ascii_uppercase();
    if !q.is_empty() && !e.callsign.to_ascii_uppercase().contains(&q) {
      return false;
    }
    if let Some(s) = qsl_filter.get() {
      let ok = match s {
        QslStatus::Confirmed => e.qsl_rcvd,
        QslStatus::SentPending => e.qsl_sent && !e.qsl_rcvd,
        QslStatus::NotSent => !e.qsl_sent && !e.qsl_rcvd,
      };
      if !ok {
        return false;
      }
    }
    true
  };

  // 筛选后的记录
  let entries_for_filter = Arc::clone(&entries);
  let filtered = Memo::new(move |_| {
    entries_for_filter
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
      .filter_map(|e| e.entity().map(|x| x.name).map(|s| s.to_owned()))
      .collect::<HashSet<_>>()
      .len()
  });
  let qsl_confirmed_count =
    Memo::new(move |_| filtered.get().iter().filter(|e| e.qsl_rcvd).count());

  // ---- 选中状态：field 展开 / square 明细 / 搜索高亮 ----
  let selected = RwSignal::new(None::<(usize, usize)>);
  let selected_square = RwSignal::new(None::<(usize, usize)>);
  let highlight = RwSignal::new(None::<String>);
  let search_input = RwSignal::new(String::new());
  let show_grayline = RwSignal::new(false);
  // 外部定位：搜索命中时聚焦视图。
  let focus = RwSignal::new(None::<(f64, f64)>);
  // 当前缩放级别（由 MapView 同步），用于按 zoom 切换 field/square 层级。
  let zoom = RwSignal::new(1.0);
  // 悬停/点击处经纬度（由 MapView 同步），用于悬停联动与点击反查。
  let hover_pos = RwSignal::new(None::<(f64, f64)>);
  let clicked_pos = RwSignal::new(None::<(f64, f64)>);

  // 本台经纬度（可选，非空网格时），用于方位/距离与大圆航线。
  let home_pos = lat_lon_from_grid(&station_grid);

  // 通联路径：本台 → 每个通联网格的大圆航线（按网格 + 波段去重），按波段着色。
  let qso_paths = Memo::new(move |_| {
    let (Some(home), true) = (home_pos, show_paths.get()) else {
      return (Vec::new(), Vec::new(), 0);
    };
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    let mut legend: Vec<String> = Vec::new();
    let mut skipped = 0;
    for e in filtered.get() {
      let Some(target) = lat_lon_from_grid(&e.gridsquare) else {
        continue;
      };
      let band = e.band_label();
      if !seen.insert((e.gridsquare.to_ascii_uppercase(), band.clone())) {
        continue;
      }
      if paths.len() >= MAX_PATHS {
        skipped += 1;
        continue;
      }
      if !legend.contains(&band) {
        legend.push(band.clone());
      }
      let title = format!("{} · {} · {}", e.callsign, e.gridsquare, band);
      paths.push((great_circle_d(home, target, 48), band_color(&band), title));
    }
    legend.sort_by_key(|b| BAND_ORDER.iter().position(|x| x == b).unwrap_or(usize::MAX));
    (paths, legend, skipped)
  });

  // 全部已通联 square（不随筛选变化），用于猎取进度与邻近推荐。
  let worked_squares = {
    let all = Arc::clone(&entries);
    let mut set = HashSet::new();
    for e in all.iter() {
      if let Some(idx) = square_index(&e.gridsquare) {
        set.insert(idx);
      }
    }
    set
  };
  // 全球已通联 square 数量（用于全球猎取进度条）。
  let world_done = worked_squares.len();
  // 通联最多的 square（全量，Top 榜）。
  let top_squares = {
    let mut m: HashMap<(usize, usize), usize> = HashMap::new();
    for e in entries.iter() {
      if let Some(idx) = square_index(&e.gridsquare) {
        *m.entry(idx).or_default() += 1;
      }
    }
    let mut v: Vec<((usize, usize), usize)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.into_iter().take(5).collect::<Vec<_>>()
  };
  // 本台 field（20°×10°）内已通联 square，用于猎取进度条。
  let hunt_done = {
    let mut set = HashSet::new();
    if let Some((fl, fa)) = field_index(&station_grid) {
      for (sl, sa) in &worked_squares {
        if sl / 10 == fl && sa / 10 == fa {
          set.insert((*sl, *sa));
        }
      }
    }
    set
  };
  // 邻近未通联推荐：本台 square 8 邻域中尚未通联的 square，按距离由近到远排序。
  let neighbor_hits = {
    let mut out = Vec::new();
    if let Some((sl, sa)) = square_index(&station_grid) {
      for dl in -1..=1isize {
        for da in -1..=1isize {
          if dl == 0 && da == 0 {
            continue;
          }
          let nl = sl as isize + dl;
          let na = sa as isize + da;
          if nl < 0 || na < 0 || nl >= 180 || na >= 180 {
            continue;
          }
          let key = (nl as usize, na as usize);
          if !worked_squares.contains(&key) {
            out.push(key);
          }
        }
      }
    }
    if let Some((hlat, hlon)) = home_pos {
      out.sort_by(|a, b| {
        let (da, _) = distance_bearing(hlat, hlon, square_center(*a).0, square_center(*a).1);
        let (db, _) = distance_bearing(hlat, hlon, square_center(*b).0, square_center(*b).1);
        da.partial_cmp(&db).unwrap_or(Ordering::Equal)
      });
    }
    out
  };

  // 点击反查：把点击处经纬度解析为网格，显示是否已通联。
  let click_inspect = Memo::new(move |_| {
    clicked_pos.get().map(|(lat, lon)| {
      let grid = grid_from_lat_lon(lat, lon).unwrap_or_default();
      let sq = square_index(&grid);
      let label = sq.map(square_label);
      let worked = sq.map(|idx| worked_squares.contains(&idx)).unwrap_or(false);
      (lat, lon, grid, label, worked)
    })
  });

  // 聚焦到指定 square（供邻近推荐点击使用）。
  let focus_square = move |key: (usize, usize)| {
    let (lat, lon) = square_center(key);
    focus.set(Some((lat, lon)));
    highlight.set(Some(square_label(key)));
  };

  // 筛选状态持久化：任一筛选变化即写入 localStorage。
  Effect::new(move |_| {
    let f = GridFilters {
      band: band_filter.get(),
      mode: mode_filter.get(),
      dxcc: dxcc_filter.get(),
      year: year_filter.get(),
      callsign: callsign_query.get(),
      qsl: qsl_filter.get(),
      paths: show_paths.get(),
    };
    crate::util::storage::set_json("grid-map-filters", &f);
  });

  // 复制当前筛选结果的统计摘要到剪贴板。
  let copy_summary = move || {
    let rows = filtered.get();
    let mut band_counts: HashMap<String, usize> = HashMap::new();
    let mut mode_counts: HashMap<String, usize> = HashMap::new();
    for e in &rows {
      let b = e.band_label();
      if !b.is_empty() {
        *band_counts.entry(b).or_default() += 1;
      }
      *mode_counts.entry(e.mode.clone()).or_default() += 1;
    }
    let mut bands_sorted: Vec<(String, usize)> = band_counts.into_iter().collect();
    bands_sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
    let mut modes_sorted: Vec<(String, usize)> = mode_counts.into_iter().collect();
    modes_sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
    let band_str = bands_sorted
      .iter()
      .map(|(b, n)| format!("{b}×{n}"))
      .collect::<Vec<_>>()
      .join(" · ");
    let mode_str = modes_sorted
      .iter()
      .map(|(m, n)| format!("{m}×{n}"))
      .collect::<Vec<_>>()
      .join(" · ");
    let text = format!(
      "通联统计\n总数 {} 条 · DXCC {} 个 · field {} · square {} · QSL 已确认 {}\n波段：{}\n模式：{}",
      rows.len(),
      dxcc_count.get(),
      field_grids.get().len(),
      square_entries.get().len(),
      qsl_confirmed_count.get(),
      band_str,
      mode_str,
    );
    copy_text(&text);
  };

  // 导出地图为 SVG 文件（注入页面样式后下载）。
  let export_svg = move |_| {
    let Some(doc) = window().document() else {
      return;
    };
    let Ok(Some(svg)) = doc.query_selector("svg[role='img']") else {
      return;
    };
    let mut svg_html = svg.outer_html();
    // 注入页面 <style>，使下载后的 SVG 保留主题配色。
    let mut css = String::new();
    if let Ok(nodes) = doc.query_selector_all("style") {
      for i in 0..nodes.length() {
        if let Some(style) = nodes.item(i) {
          css.push_str(&style.text_content().unwrap_or_default());
        }
      }
    }
    if !css.is_empty() {
      let inject = format!("<style>{css}</style>");
      if let Some(idx) = svg_html.find('>') {
        svg_html.insert_str(idx + 1, &inject);
      }
    }
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(&svg_html));
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(parts.as_ref()) else {
      return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
      return;
    };
    let Ok(a) = doc.create_element("a") else {
      return;
    };
    let _ = a.set_attribute("href", &url);
    let _ = a.set_attribute("download", "grid-map.svg");
    let a_html: web_sys::HtmlElement = a.unchecked_into();
    a_html.click();
    let _ = web_sys::Url::revoke_object_url(&url);
  };

  // ---- 灰线实时时钟（60s 刷新，与灰线页一致）----
  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  set_interval(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  );

  // 搜索定位：解析网格 → 缩放到 square 级别并居中 → 红色十字高亮。
  let do_search = move |_| {
    let g = search_input.get().trim().to_uppercase();
    if let Some((lat, lon)) = lat_lon_from_grid(&g) {
      focus.set(Some((lat, lon)));
      highlight.set(Some(g));
    } else {
      highlight.set(None);
    }
  };

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

  // ---- 叠加层（信息层）：field / square 热力 + 选中轮廓 + 家标记 + 灰线 + 交互 ----
  // 构建为可复用的 `Fn` 闭包：MapView 在反子午线三份平铺中各自调用一次。
  let overlay = move || {
    view! {
      // L3 field 热力（zoom 较小时显示）
      {move || {
        (zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
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
        })
      }}
      // L4 square 热力（含悬停 tooltip，zoom 较大时显示）
      {move || {
        (zoom.get() >= FIELD_SQUARE_ZOOM).then(|| {
          let smax = square_max.get();
          square_entries
            .get()
            .iter()
            .map(|((sl, sa), list)| {
              let x = (sl * 4).to_string();
              let y = ((179 - sa) * 2).to_string();
              let label = square_label((*sl, *sa));
              let n = list.len();
              let confirmed = list.iter().filter(|e| e.qsl_rcvd).count();
              let sent = list.iter().filter(|e| e.qsl_sent && !e.qsl_rcvd).count();
              let cls = if confirmed == n {
                square_fill_class_confirmed(n, smax)
              } else if confirmed > 0 || sent > 0 {
                square_fill_class_sent(n, smax)
              } else {
                square_fill_class(n, smax)
              };
              let (lat, lon) = square_center((*sl, *sa));
              let title = match home_pos {
                Some((hlat, hlon)) => {
                  let (d, b) = distance_bearing(hlat, hlon, lat, lon);
                  format!("{label}：{n} 条（{confirmed} 确认）· {b:.0}° / {d:.0} km")
                }
                None => format!("{label}：{n} 条（{confirmed} 确认）"),
              };
              view! {
                <rect x=x y=y width="4" height="2" class=cls>
                  <title>{title}</title>
                </rect>
              }
            })
            .collect_view()
        })
      }}
      // field 标签（zoom 较小时显示）
      {move || {
        (zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
          view! {
            <g class="fill-muted-foreground" style="opacity:0.75">
              {field_grids
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
                .collect_view()}
            </g>
          }
        })
      }}
      // 选中 field 轮廓（zoom 较小时显示）
      {move || {
        (zoom.get() < FIELD_SQUARE_ZOOM)
          .then(|| selected.get())
          .flatten()
          .map(|(fl, fa)| {
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
      // 选中 square 轮廓（zoom 较大时显示）
      {move || {
        (zoom.get() >= FIELD_SQUARE_ZOOM)
          .then(|| selected_square.get())
          .flatten()
          .map(|(sl, sa)| {
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
      // 悬停 square 描边（悬停联动，zoom 较大时显示）
      {move || {
        (zoom.get() >= FIELD_SQUARE_ZOOM)
          .then(|| {
            hover_pos.get().and_then(|(lat, lon)| {
              square_index(&grid_from_lat_lon(lat, lon).unwrap_or_default())
            })
          })
          .flatten()
          .map(|(sl, sa)| {
            let x = (sl * 4).to_string();
            let y = ((179 - sa) * 2).to_string();
            view! {
              <rect
                x=x
                y=y
                width="4"
                height="2"
                fill="none"
                class="stroke-foreground"
                stroke-width="1.5"
                opacity="0.8"
                vector-effect="non-scaling-stroke"
              />
            }
          })
      }}
      // 通联路径（全部筛选结果）
      {move || {
        qso_paths.with(|(paths, _, _)| {
          paths
            .iter()
            .map(|(d, color, title)| {
              view! {
                <path
                  d=format!("M {d}")
                  fill="none"
                  stroke=*color
                  stroke-width="1.2"
                  opacity="0.75"
                  vector-effect="non-scaling-stroke"
                >
                  <title>{title.clone()}</title>
                </path>
              }
            })
            .collect_view()
        })
      }}
      // 本台“家”标记
      {home_marker.clone()}
      // 大圆航线：选中或悬停 square 时绘制本台 → square 中心的最短路径。
      {move || {
        let target = selected_square.get().or_else(|| {
          hover_pos.get().and_then(|(lat, lon)| {
            square_index(&grid_from_lat_lon(lat, lon).unwrap_or_default())
          })
        });
        home_pos.zip(target).map(|(home, (sl, sa))| {
          let d = great_circle_d(home, square_center((sl, sa)), 128);
          view! {
            <path
              d=format!("M {d}")
              fill="none"
              class="stroke-primary"
              stroke-width="1.5"
              stroke-dasharray="3 3"
              opacity="0.7"
              vector-effect="non-scaling-stroke"
            />
          }
        })
      }}
      // 灰线叠加（晨昏圈 + 夜半球遮罩）
      {move || {
        show_grayline.get().then(|| {
          view! { <GraylineOverlay now_ms=now /> }
        })
      }}
      // field 交互层（点击展开，zoom 较小时显示）
      {move || {
        (zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
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
        })
      }}
      // square 交互层（点击展开明细，覆盖在 field 交互层之上，zoom 较大时显示）
      {move || {
        (zoom.get() >= FIELD_SQUARE_ZOOM).then(|| {
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
        })
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
    }
  };

  view! {
    <div>
      // 空状态：无通联记录时提示
      {entries_empty.then(|| {
        view! {
          <div class="mb-3 rounded-lg border bg-muted/30 p-4 text-center text-sm text-muted-foreground">
            "暂无通联记录，请先在「通联日志」中添加记录，或从 ADIF / CSV 导入。"
          </div>
        }
      })}
      // 工具栏：搜索定位 + 波段 / 模式筛选 + 灰线开关
      <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
        <div class="flex items-center gap-1">
          <input
            type="text"
            placeholder="定位网格，如 OM89EW"
            aria-label="定位网格"
            class=input_class("h-8 w-36 font-mono uppercase")
            prop:value=move || search_input.get()
            on:input=move |e| search_input.set(event_target_value(&e).to_uppercase())
          />
          <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=do_search>
            "定位"
          </button>
        </div>
        <input
          type="text"
          placeholder="呼号搜索"
          aria-label="呼号搜索"
          class=input_class("h-8 w-32 font-mono uppercase")
          prop:value=move || callsign_query.get()
          on:input=move |e| callsign_query.set(event_target_value(&e).to_uppercase())
        />
        <select
          aria-label="波段筛选"
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
          aria-label="模式筛选"
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
        <select
          aria-label="实体筛选"
          class=input_class("h-8 w-auto")
          on:change=move |e| {
            let v = event_target_value(&e);
            dxcc_filter.set(if v.is_empty() { None } else { Some(v) });
          }
        >
          <option value="">"全部实体"</option>
          {dxcc_list
            .iter()
            .map(|d| {
              view! { <option value=d.clone()>{d.clone()}</option> }
            })
            .collect_view()}
        </select>
        <select
          aria-label="年份筛选"
          class=input_class("h-8 w-auto")
          on:change=move |e| {
            let v = event_target_value(&e);
            year_filter.set(if v.is_empty() { None } else { Some(v) });
          }
        >
          <option value="">"全部年份"</option>
          {year_list
            .iter()
            .map(|y| {
              view! { <option value=y.clone()>{y.clone()}</option> }
            })
            .collect_view()}
        </select>
        <select
          aria-label="QSL 筛选"
          class=input_class("h-8 w-auto")
          on:change=move |e| {
            let v = event_target_value(&e);
            qsl_filter.set(match v.as_str() {
              "confirmed" => Some(QslStatus::Confirmed),
              "sent_pending" => Some(QslStatus::SentPending),
              "not_sent" => Some(QslStatus::NotSent),
              _ => None,
            });
          }
        >
          <option value="">"全部 QSL"</option>
          <option value="confirmed">"已确认"</option>
          <option value="sent_pending">"已寄未确认"</option>
          <option value="not_sent">"未寄出"</option>
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
        <label
          class="flex cursor-pointer items-center gap-1.5 text-muted-foreground"
          title=if home_pos.is_some() { "从本台到每个通联网格的大圆路径" } else { "需先在本台信息中填写网格" }
        >
          <input
            type="checkbox"
            class="size-4 accent-primary"
            prop:disabled=home_pos.is_none()
            prop:checked=move || show_paths.get()
            on:change=move |e| show_paths.set(event_target_checked(&e))
          />
          "通联路径"
        </label>
      </div>

      // 统计概览徽章 + 复制摘要
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
        <span class="rounded-full border bg-muted/40 px-2 py-0.5">
          "QSL "
          <span class="font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
            {move || qsl_confirmed_count.get()}
          </span>
        </span>
        <div class="ml-auto flex items-center gap-1">
          <button
            type="button"
            class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
            on:click=export_svg
          >
            "导出 SVG"
          </button>
          <button
            type="button"
            class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
            on:click=move |_| copy_summary()
          >
            "复制摘要"
          </button>
        </div>
      </div>

      <MapView
        aria_label="已通联网格地图（滚轮缩放、拖拽平移、双指缩放、双击复位、反子午线环绕）"
        focus=focus
        zoom_signal=zoom
        hover_signal=hover_pos
        click_signal=clicked_pos
      >
        {overlay.clone()}
      </MapView>

      // 网格猎取进度（本台 field 内共 100 个 square）
      {move || {
        field_index(&station_grid).map(|(fl, fa)| {
          let f_lon = (b'A' + fl as u8) as char;
          let f_lat = (b'A' + fa as u8) as char;
          let done = hunt_done.len();
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3">
              <div class="flex items-center justify-between text-xs">
                <span class="font-medium">"网格猎取（field " {f_lon}{f_lat} "）"</span>
                <span class="tabular-nums text-muted-foreground">{done} " / 100"</span>
              </div>
              <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary transition-all"
                  style=format!("width: {}%", done)
                ></div>
              </div>
            </div>
          }
        })
      }}

      // 全球 square 猎取进度（全部 32400 个 square）
      {move || {
        let pct = world_done as f64 / WORLD_SQUARES as f64 * 100.0;
        view! {
          <div class="mt-2 rounded-lg border bg-muted/30 p-3">
            <div class="flex items-center justify-between text-xs">
              <span class="font-medium">"全球 square 猎取"</span>
              <span class="tabular-nums text-muted-foreground">{world_done} " / " {WORLD_SQUARES}</span>
            </div>
            <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                class="h-full rounded-full bg-primary transition-all"
                style=format!("width: {:.3}%", pct)
              ></div>
            </div>
          </div>
        }
      }}

      // 通联最多 square（Top 榜，全量）
      {(!top_squares.is_empty()).then(|| {
        view! {
          <div class="mt-2 rounded-lg border bg-muted/30 p-3 text-xs">
            <div class="mb-1.5 font-medium">"通联最多 square"</div>
            <div class="flex flex-wrap gap-1.5">
              {top_squares
                .iter()
                .map(|((sl, sa), n)| {
                  let label = square_label((*sl, *sa));
                  view! {
                    <span class="rounded bg-muted/60 px-2 py-0.5 font-mono text-xs">{format!("{label} ×{n}")}</span>
                  }
                })
                .collect_view()}
            </div>
          </div>
        }
      })}

      // 邻近未通联推荐（本台 square 8 邻域，按距离排序）
      {move || {
        (!neighbor_hits.is_empty()).then(|| {
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3">
              <div class="mb-1.5 text-xs font-medium">"邻近未通联（点击定位）"</div>
              <div class="flex flex-wrap gap-1.5">
                {neighbor_hits
                  .iter()
                  .map(|key| {
                    let k = *key;
                    let label = square_label(k);
                    let (meta, title) = match home_pos {
                      Some((hlat, hlon)) => {
                        let (d, b) =
                          distance_bearing(hlat, hlon, square_center(k).0, square_center(k).1);
                        (format!("{label} {b:.0}°"), format!("{label} · 距离 {d:.0} km"))
                      }
                      None => (label.clone(), label.clone()),
                    };
                    view! {
                      <button
                        type="button"
                        class="rounded bg-muted/60 px-2 py-0.5 font-mono text-xs transition-colors hover:bg-accent"
                        on:click=move |_| focus_square(k)
                        title=title
                      >
                        {meta}
                      </button>
                    }
                  })
                  .collect_view()}
              </div>
            </div>
          }
        })
      }}

      // 点击反查结果
      {move || {
        click_inspect.get().map(|(lat, lon, grid, label, worked)| {
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3 text-xs">
              <div class="mb-1 flex items-center justify-between">
                <span class="font-semibold">"点击反查"</span>
                <button
                  type="button"
                  class="text-muted-foreground transition-colors hover:text-foreground"
                  on:click=move |_| clicked_pos.set(None)
                >
                  "关闭"
                </button>
              </div>
              <div class="font-mono text-sm">{grid}</div>
              <div class="mt-0.5 text-muted-foreground tabular-nums">
                {format!("{lat:.3}°, {lon:.3}°")}
              </div>
              <div class="mt-0.5">
                {match (label, worked) {
                  (Some(l), true) => format!("square {l}：已通联"),
                  (Some(l), false) => format!("square {l}：未通联"),
                  (None, _) => "该点无法解析为网格".to_string(),
                }}
              </div>
            </div>
          }
        })
      }}

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
        {move || {
          qso_paths.with(|(_, legend, skipped)| {
            (!legend.is_empty()).then(|| {
              let skipped = *skipped;
              view! {
                <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
                  "路径"
                  {legend
                    .iter()
                    .map(|b| {
                      view! {
                        <span class="inline-flex items-center gap-1">
                          <span class="inline-block h-0.5 w-4 rounded" style=format!("background:{}", band_color(b))></span>
                          {if b.is_empty() { "未知".to_owned() } else { b.clone() }}
                        </span>
                      }
                    })
                    .collect_view()}
                  {(skipped > 0).then(|| format!("（另有 {skipped} 条未绘制）"))}
                </span>
              }
            })
          })
        }}
        <span class="text-muted-foreground">"滚轮/双指缩放 · 拖拽平移 · 双击复位 · 输入网格定位 · 点击 field/square 展开"</span>
      </div>

      // 选中 field 的展开列表
      {move || {
        selected.get().map(|(fl, fa)| {
          let f_lon = (b'A' + fl as u8) as char;
          let f_lat = (b'A' + fa as u8) as char;
          let mut list = field_grids.get().get(&(fl, fa)).cloned().unwrap_or_default();
          list.sort();
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
          let mut list = square_entries.get().get(&(sl, sa)).cloned().unwrap_or_default();
          list.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.time.cmp(&a.time)));
          let confirmed = list.iter().filter(|e| e.qsl_rcvd).count();
          let (lat, lon) = square_center((sl, sa));
          let bearing_line = home_pos.map(|(hlat, hlon)| {
            let (d, b) = distance_bearing(hlat, hlon, lat, lon);
            format!("本台方位 {b:.0}° · 距离 {d:.0} km")
          });
          view! {
            <div class="mt-2 rounded-lg border bg-muted/30 p-3">
              <div class="mb-2 flex items-center justify-between text-xs">
                <span class="font-semibold">
                  {if confirmed > 0 {
                    format!("网格 {label} · {} 条 · {confirmed} 已确认", list.len())
                  } else {
                    format!("网格 {label} · {} 条", list.len())
                  }}
                </span>
                <button
                  type="button"
                  class="text-muted-foreground transition-colors hover:text-foreground"
                  on:click=move |_| selected_square.set(None)
                >
                  "关闭"
                </button>
              </div>
              {bearing_line.map(|line| {
                view! {
                  <div class="mb-2 text-xs tabular-nums text-muted-foreground">{line}</div>
                }
              })}
              <div class="flex flex-wrap gap-1.5">
                {list
                  .iter()
                  .map(|e| {
                    let band = Some(e.band_label()).filter(|b| !b.is_empty()).unwrap_or_else(|| "—".to_owned());
                    // 显示完整网格码：6 位时体现 subsquare，4 位时仅 square。
                    let grid = e.gridsquare.trim().to_ascii_uppercase();
                    let entity = e.entity().map(|x| x.name).unwrap_or("");
                    let rst = if e.rst_sent.is_empty() && e.rst_rcvd.is_empty() {
                      String::new()
                    } else {
                      let s = if e.rst_sent.is_empty() { "—".to_string() } else { e.rst_sent.clone() };
                      let r = if e.rst_rcvd.is_empty() { "—".to_string() } else { e.rst_rcvd.clone() };
                      format!("RST {s}/{r}")
                    };
                    let span_class = if e.qsl_rcvd {
                      "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs text-emerald-600 dark:text-emerald-400"
                    } else if e.qsl_sent {
                      "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs text-amber-600 dark:text-amber-400"
                    } else {
                      "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs"
                    };
                    let mut detail = format!("{} · {} · {} · {}", e.callsign, grid, band, e.mode);
                    if !rst.is_empty() {
                      detail.push_str(&format!(" · {rst}"));
                    }
                    if !e.name.is_empty() {
                      detail.push_str(&format!(" · {}", e.name));
                    }
                    if !e.qth.is_empty() {
                      detail.push_str(&format!(" · {}", e.qth));
                    }
                    detail.push_str(&format!(" · {}", e.date));
                    if !entity.is_empty() {
                      detail.push_str(&format!(" · {entity}"));
                    }
                    let tooltip = format!(
                      "时间 {} · 备注 {}",
                      if e.time.is_empty() { "—".to_string() } else { e.time.clone() },
                      if e.remark.is_empty() { "—".to_string() } else { e.remark.clone() },
                    );
                    view! {
                      <span class=span_class title=tooltip>{detail}</span>
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

#[cfg(test)]
mod tests {

  use super::super::grid_fill::{field_fill_class, square_fill_class};
  use super::super::grid_geo::square_label;

  #[test]
  fn square_label_formats_field_and_square() {
    // (sl, sa) 全局索引 → 4 位网格码。
    assert_eq!(square_label((14 * 10 + 8, 12 * 10 + 9)), "OM89");
    assert_eq!(square_label((9 * 10, 9 * 10)), "JJ00");
  }

  #[test]
  fn fill_classes_bucket_by_ratio() {
    assert_eq!(field_fill_class(0, 100), "fill-primary/15");
    assert_eq!(field_fill_class(100, 100), "fill-primary/60");
    assert_eq!(square_fill_class(0, 100), "fill-primary/30");
    assert_eq!(square_fill_class(100, 100), "fill-primary");
  }
}
