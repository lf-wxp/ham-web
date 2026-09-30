use std::collections::HashMap;
use std::sync::Arc;

use ham_web_core::grid::{field_index, lat_lon_from_grid, square_index};
use leptos::prelude::*;

/// 简化大陆轮廓（经纬度，粗粒度，用于地图背景定位）。
pub const CONTINENTS: &[&[(f64, f64)]] = &[
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
];

/// 等距圆柱投影：经纬度 → SVG 坐标。
pub fn project(lon: f64, lat: f64) -> (f64, f64) {
  ((lon + 180.0) * 2.0, (90.0 - lat) * 2.0)
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

/// Maidenhead 网格地图：等距圆柱投影，square（2°×1°）精细热力 + 大陆轮廓 + field 点击展开。
///
/// `highlight` 为可选的高亮查询网格（响应式），在地图上以红色标记定位其中心。
#[component]
pub fn GridMap(grids: Vec<String>, highlight: Signal<Option<String>>) -> impl IntoView {
  let mut field_grids: HashMap<(usize, usize), Vec<String>> = HashMap::new();
  let mut square_counts: HashMap<(usize, usize), usize> = HashMap::new();
  for g in &grids {
    if let Some(idx) = field_index(g) {
      field_grids.entry(idx).or_default().push(g.clone());
    }
    if let Some(idx) = square_index(g) {
      *square_counts.entry(idx).or_default() += 1;
    }
  }
  let field_grids = Arc::new(field_grids);
  let max = square_counts.values().copied().max().unwrap_or(1);
  let field_max = field_grids.values().map(|v| v.len()).max().unwrap_or(1);
  let selected = RwSignal::new(None::<(usize, usize)>);

  view! {
    <div>
      <svg
        viewBox="0 0 720 360"
        class="w-full text-primary"
        role="img"
        aria-label="已通联网格地图"
        preserveAspectRatio="xMidYMid meet"
      >
        <rect x="0" y="0" width="720" height="360" fill="currentColor" opacity="0.04" />
        {CONTINENTS
          .iter()
          .map(|poly| {
            let pts = polygon_points(poly);
            view! { <polygon points=pts fill="currentColor" opacity="0.10" stroke="none" /> }
          })
          .collect_view()}
        {field_grids
          .iter()
          .map(|((fl, fa), list)| {
            let x = (fl * 40).to_string();
            let y = ((17 - fa) * 20).to_string();
            let opacity = (0.15 + 0.25 * (list.len() as f64 / field_max as f64)).to_string();
            view! { <rect x=x y=y width="40" height="20" fill="currentColor" opacity=opacity /> }
          })
          .collect_view()}
        {(0..=18)
          .map(|i| {
            let x = (i * 40).to_string();
            view! {
              <line x1=x.clone() y1="0" x2=x y2="360" stroke="currentColor" stroke-opacity="0.10" stroke-width="1" />
            }
          })
          .collect_view()}
        {(0..=18)
          .map(|i| {
            let y = (i * 20).to_string();
            view! {
              <line x1="0" y1=y.clone() x2="720" y2=y stroke="currentColor" stroke-opacity="0.10" stroke-width="1" />
            }
          })
          .collect_view()}
        <line x1="0" y1="180" x2="720" y2="180" stroke="currentColor" stroke-opacity="0.25" stroke-width="1" />
        <line x1="360" y1="0" x2="360" y2="360" stroke="currentColor" stroke-opacity="0.25" stroke-width="1" />
        {square_counts
          .iter()
          .map(|((sl, sa), n)| {
            let x = (sl * 4).to_string();
            // Maidenhead 纬度索引 0（A）在最南，SVG y 轴向下，需要翻转。
            let y = ((179 - sa) * 2).to_string();
            let opacity = (0.3 + 0.7 * (*n as f64 / max as f64)).to_string();
            view! { <rect x=x y=y width="4" height="2" fill="currentColor" opacity=opacity /> }
          })
          .collect_view()}
        {field_grids
          .iter()
          .map(|((fl, fa), list)| {
            let x = (fl * 40).to_string();
            let y = ((17 - fa) * 20).to_string();
            let f_lon = (b'A' + *fl as u8) as char;
            let f_lat = (b'A' + *fa as u8) as char;
            let key = (*fl, *fa);
            view! {
              <rect
                x=x
                y=y
                width="40"
                height="20"
                fill="transparent"
                class="cursor-pointer"
                on:click=move |_| selected.set(Some(key))
              >
                <title>{format!("{f_lon}{f_lat}：{} 个网格，点击查看", list.len())}</title>
              </rect>
            }
          })
          .collect_view()}
        {move || {
          highlight.get().map(|g| {
            let (lat, lon) = lat_lon_from_grid(&g).unwrap_or((0.0, 0.0));
            let (x, y) = project(lon, lat);
            view! {
              <g>
                <circle
                  cx=x.to_string()
                  cy=y.to_string()
                  r="8"
                  fill="none"
                  stroke="#ef4444"
                  stroke-width="2.5"
                />
                <line
                  x1=(x - 11.0).to_string()
                  y1=y.to_string()
                  x2=(x + 11.0).to_string()
                  y2=y.to_string()
                  stroke="#ef4444"
                  stroke-width="2"
                />
                <line
                  x1=x.to_string()
                  y1=(y - 11.0).to_string()
                  x2=x.to_string()
                  y2=(y + 11.0).to_string()
                  stroke="#ef4444"
                  stroke-width="2"
                />
              </g>
            }
          })
        }}
      </svg>
      {move || {
        selected.get().map(|(fl, fa)| {
          let f_lon = (b'A' + fl as u8) as char;
          let f_lat = (b'A' + fa as u8) as char;
          let list = field_grids.get(&(fl, fa)).cloned().unwrap_or_default();
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
    </div>
  }
}
