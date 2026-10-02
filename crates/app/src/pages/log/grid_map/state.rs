//! 网格地图响应式状态：筛选、聚合、静态统计与搜索 / 导出等操作集中于此，供各子组件共享。

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::grid::{
  distance_bearing, field_index, grid_from_lat_lon, lat_lon_from_grid, square_index,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::pages::log::LogEntry;
use crate::util::copy_text;

use super::super::grid_fill::band_color;
use super::super::grid_filter::{GridFilters, QslStatus};
use super::super::grid_geo::{great_circle_d, square_center, square_label};
use crate::i18n::tf;

/// 波段显示顺序（低频 → 高频），筛选下拉据此排序。
pub(super) const BAND_ORDER: &[&str] = &[
  "160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm",
];

/// 缩放阈值：`zoom` 达到该值后，地图热力由 field（20°×10°）切换为 square（2°×1°）。
pub(super) const FIELD_SQUARE_ZOOM: f64 = 4.0;

/// 全球 square 总数（180 经度 × 180 纬度 = 2°×1° 网格）。
pub(super) const WORLD_SQUARES: usize = 180 * 180;

/// 通联路径最多绘制条数（按网格 + 波段去重后），避免大日志拖慢渲染。
const MAX_PATHS: usize = 600;

/// 通联路径集合：`(SVG path, 颜色, tooltip)` 列表 + 图例波段 + 未绘制条数。
pub(super) type QsoPaths = (Vec<(String, &'static str, String)>, Vec<String>, usize);

/// 点击反查结果：`(纬度, 经度, 网格, square 标签, 是否已通联)`。
pub(super) type ClickInspect = (f64, f64, String, Option<String>, bool);

/// 一次性计算、不随筛选变化的静态数据。
#[derive(Clone)]
pub(super) struct GridStatic {
  pub(super) bands: Vec<String>,
  pub(super) mode_list: Vec<String>,
  pub(super) dxcc_list: Vec<String>,
  pub(super) year_list: Vec<String>,
  pub(super) world_done: usize,
  pub(super) top_squares: Vec<((usize, usize), usize)>,
  pub(super) hunt_done: HashSet<(usize, usize)>,
  pub(super) neighbor_hits: Vec<(usize, usize)>,
  pub(super) home_pos: Option<(f64, f64)>,
  pub(super) station_grid: String,
  pub(super) entries_empty: bool,
}

/// 网格地图共享状态（信号 + 派生 Memo + 静态数据），跨子组件按值传递。
#[derive(Clone, Copy)]
pub(super) struct GridMapState {
  pub(super) band_filter: RwSignal<Option<String>>,
  pub(super) mode_filter: RwSignal<Option<String>>,
  pub(super) dxcc_filter: RwSignal<Option<String>>,
  pub(super) year_filter: RwSignal<Option<String>>,
  pub(super) callsign_query: RwSignal<String>,
  pub(super) qsl_filter: RwSignal<Option<QslStatus>>,
  pub(super) show_paths: RwSignal<bool>,
  pub(super) show_grayline: RwSignal<bool>,
  pub(super) search_input: RwSignal<String>,
  pub(super) selected: RwSignal<Option<(usize, usize)>>,
  pub(super) selected_square: RwSignal<Option<(usize, usize)>>,
  pub(super) highlight: RwSignal<Option<String>>,
  pub(super) focus: RwSignal<Option<(f64, f64)>>,
  pub(super) zoom: RwSignal<f64>,
  pub(super) hover_pos: RwSignal<Option<(f64, f64)>>,
  pub(super) clicked_pos: RwSignal<Option<(f64, f64)>>,
  pub(super) now: RwSignal<f64>,
  pub(super) filtered: Memo<Vec<LogEntry>>,
  pub(super) field_grids: Memo<HashMap<(usize, usize), Vec<String>>>,
  pub(super) square_entries: Memo<HashMap<(usize, usize), Vec<LogEntry>>>,
  pub(super) field_max: Memo<usize>,
  pub(super) square_max: Memo<usize>,
  pub(super) grid_count: Memo<usize>,
  pub(super) dxcc_count: Memo<usize>,
  pub(super) qsl_confirmed_count: Memo<usize>,
  pub(super) qso_paths: Memo<QsoPaths>,
  pub(super) click_inspect: Memo<Option<ClickInspect>>,
  pub(super) data: StoredValue<GridStatic>,
}

impl GridMapState {
  pub(super) fn new(entries: Vec<LogEntry>, station_grid: String) -> Self {
    let entries = Arc::new(entries);
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

    // ---- 灰线实时时钟（60s 刷新，与灰线页一致）----
    let now = RwSignal::new(js_sys::Date::new_0().get_time());
    set_interval(
      move || now.set(js_sys::Date::new_0().get_time()),
      Duration::from_secs(60),
    );

    Self {
      band_filter,
      mode_filter,
      dxcc_filter,
      year_filter,
      callsign_query,
      qsl_filter,
      show_paths,
      show_grayline,
      search_input,
      selected,
      selected_square,
      highlight,
      focus,
      zoom,
      hover_pos,
      clicked_pos,
      now,
      filtered,
      field_grids,
      square_entries,
      field_max,
      square_max,
      grid_count,
      dxcc_count,
      qsl_confirmed_count,
      qso_paths,
      click_inspect,
      data: StoredValue::new(GridStatic {
        bands,
        mode_list,
        dxcc_list,
        year_list,
        world_done,
        top_squares,
        hunt_done,
        neighbor_hits,
        home_pos,
        station_grid,
        entries_empty,
      }),
    }
  }

  /// 搜索定位：解析网格 → 缩放到 square 级别并居中 → 红色十字高亮。
  pub(super) fn do_search(&self) {
    let g = self.search_input.get().trim().to_uppercase();
    if let Some((lat, lon)) = lat_lon_from_grid(&g) {
      self.focus.set(Some((lat, lon)));
      self.highlight.set(Some(g));
    } else {
      self.highlight.set(None);
    }
  }

  /// 聚焦到指定 square（供邻近推荐点击使用）。
  pub(super) fn focus_square(&self, key: (usize, usize)) {
    let (lat, lon) = square_center(key);
    self.focus.set(Some((lat, lon)));
    self.highlight.set(Some(square_label(key)));
  }

  /// 复制当前筛选结果的统计摘要到剪贴板。
  pub(super) fn copy_summary(&self) {
    let rows = self.filtered.get();
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
    let text = tf(
      "通联统计\n总数 {} 条 · DXCC {} 个 · field {} · square {} · QSL 已确认 {}\n波段：{}\n模式：{}",
      &[
        &(rows.len()).to_string(),
        &(self.dxcc_count.get()).to_string(),
        &(self.field_grids.get().len()).to_string(),
        &(self.square_entries.get().len()).to_string(),
        &(self.qsl_confirmed_count.get()).to_string(),
        &(band_str).to_string(),
        &(mode_str).to_string(),
      ],
    );
    copy_text(&text);
  }

  /// 导出地图为 SVG 文件（注入页面样式后下载）。
  pub(super) fn export_svg(&self) {
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
  }
}
