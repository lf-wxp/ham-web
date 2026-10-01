//! 灰线地图：实时晨昏圈（日出/日落分界），用于判断低频 DX 的灰线窗口。
//!
//! 底图与 [`super::grid_map`] 共享同一套绘制方案：
//! - 等距圆柱投影（`project`），陆地轮廓统一经 `simplify` 简化并描边；
//! - 主题 token 分层着色（海洋 `fill-muted/40`、陆地 `fill-foreground/10`、经纬网 `stroke-foreground/10`）；
//! - 支持滚轮缩放、拖拽平移、双击/按钮复位，经纬度刻度与大陆/海洋名称标注。
//!
//! 差异仅在信息层：本图叠加**晨昏圈**（琥珀色）与**太阳直射点**，替代通联热力。

use std::time::Duration;

use leptos::prelude::*;
use leptos::svg;
use wasm_bindgen::JsValue;

use crate::pages::log::{CONTINENTS, COUNTRY_LABELS, polygon_points, project, simplify};
use crate::util::set_title;

/// 等距圆柱投影比例：经度每度 → `2` 像素。
const K: f64 = 2.0;
const MAP_W: f64 = 720.0;
const MAP_H: f64 = 360.0;

/// 一年中的第几天（1–366）。
pub fn day_of_year(year: u32, month: u32, day: u32) -> u32 {
  let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
  let cum = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
  let mut doy = cum[(month - 1) as usize] + day;
  if leap && month > 2 {
    doy += 1;
  }
  doy
}

/// 太阳赤纬（度），近似公式。
pub fn solar_declination(day_of_year: u32) -> f64 {
  23.44 * (2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.25).sin()
}

/// 太阳直射点经度（度，东经为正），由 UTC 小时推算。
pub fn subsolar_longitude(utc_hour: f64) -> f64 {
  15.0 * (12.0 - utc_hour)
}

/// 某经度处的晨昏圈纬度（度）。赤纬接近 0 时做限幅避免除零。
fn terminator_lat(lon: f64, decl: f64, subsolar_lon: f64) -> f64 {
  let d = if decl.abs() < 1.0 {
    1.0 * decl.signum()
  } else {
    decl
  };
  let h = (lon - subsolar_lon).to_radians();
  (-h.cos() / d.to_radians().tan()).atan().to_degrees()
}

/// 构建灰线带闭合路径（晨昏圈 ± `half_band` 度）。
pub fn band_path(decl: f64, subsolar_lon: f64, half_band: f64) -> String {
  let mut upper: Vec<(f64, f64)> = Vec::new();
  let mut lower: Vec<(f64, f64)> = Vec::new();
  // 步长 2px（1° 经度）保证放大后仍平滑。
  for x in (0..=720).step_by(2) {
    let lon = x as f64 / 2.0 - 180.0;
    let lat = terminator_lat(lon, decl, subsolar_lon);
    let (px, py) = project(lon, lat);
    upper.push((px, py - half_band * 2.0));
    lower.push((px, py + half_band * 2.0));
  }
  let mut d = String::new();
  for (i, (x, y)) in upper.iter().enumerate() {
    d.push_str(if i == 0 { "M" } else { "L" });
    d.push_str(&format!("{x:.1} {y:.1}"));
  }
  for (x, y) in lower.iter().rev() {
    d.push_str(&format!("L{x:.1} {y:.1}"));
  }
  d.push('Z');
  d
}

/// 夜半球闭合路径：晨昏圈曲线 + 地图边界围成的暗色区域。
/// 太阳直射北半球（`decl >= 0`）时夜在南极侧，反之夜在北极侧。
pub fn night_path(decl: f64, sslon: f64) -> String {
  let mut d = String::new();
  let mut first = true;
  for x in (0..=720).step_by(2) {
    let lon = x as f64 / 2.0 - 180.0;
    let lat = terminator_lat(lon, decl, sslon);
    let (px, py) = project(lon, lat);
    d.push_str(if first { "M" } else { "L" });
    first = false;
    d.push_str(&format!("{px:.1} {py:.1}"));
  }
  if decl >= 0.0 {
    d.push_str("L720.0 360.0L0.0 360.0Z");
  } else {
    d.push_str("L720.0 0.0L0.0 0.0Z");
  }
  d
}

/// 两个活动指针的欧氏距离（像素），用于双指捏合缩放。
fn ptr_dist(a: &(i32, f64, f64), b: &(i32, f64, f64)) -> f64 {
  let dx = a.1 - b.1;
  let dy = a.2 - b.2;
  (dx * dx + dy * dy).sqrt()
}

#[component]
pub fn GraylinePage() -> impl IntoView {
  set_title("灰线地图");

  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  // 时间偏移（小时）：0 = 当前，正数预测未来、负数回溯过去
  let time_offset = RwSignal::new(0.0f64);
  set_interval(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  );

  let sun = Memo::new(move |_| {
    let ms = now.get() + time_offset.get() * 3_600_000.0;
    let date = js_sys::Date::new(&JsValue::from_f64(ms));
    let y = date.get_utc_full_year();
    let mo = date.get_utc_month() + 1;
    let d = date.get_utc_date();
    let doy = day_of_year(y, mo, d);
    let hour = date.get_utc_hours() as f64
      + date.get_utc_minutes() as f64 / 60.0
      + date.get_utc_seconds() as f64 / 3600.0;
    (doy, hour)
  });

  // 当前显示的 UTC 时钟（含时间偏移）
  let clock = Memo::new(move |_| {
    let ms = now.get() + time_offset.get() * 3_600_000.0;
    let date = js_sys::Date::new(&JsValue::from_f64(ms));
    format!(
      "{:02}:{:02}:{:02} UTC",
      date.get_utc_hours(),
      date.get_utc_minutes(),
      date.get_utc_seconds()
    )
  });

  // ---- 缩放/平移状态（与网格地图一致）----
  let view_box = RwSignal::new((0.0f64, 0.0f64, MAP_W, MAP_H));
  // 缩放倍率：1 = 全图，>1 表示放大（用于国家标签分级显示）。
  let zoom = Signal::derive(move || MAP_W / view_box.get().2);
  let dragging = RwSignal::new(false);
  let drag_start = StoredValue::new((0.0f64, 0.0f64, 0.0f64, 0.0f64));
  let svg_ref = NodeRef::<svg::Svg>::new();
  // 双指捏合所需的指针追踪
  let active_pointers = RwSignal::new(Vec::<(i32, f64, f64)>::new());
  let pinch_state = StoredValue::new((0.0f64, (0.0f64, 0.0f64, MAP_W, MAP_H)));
  // 悬停经纬度提示：(css_x, css_y, lon, lat)
  let hover = RwSignal::new(None::<(f64, f64, f64, f64)>);

  let zoom_by = move |factor: f64| {
    let (x, y, w, h) = view_box.get();
    let nw = (w * factor).clamp(MAP_W / 16.0, MAP_W);
    let nh = (h * factor).clamp(MAP_H / 16.0, MAP_H);
    let nx = (x + (w - nw) / 2.0).clamp(0.0, MAP_W - nw);
    let ny = (y + (h - nh) / 2.0).clamp(0.0, MAP_H - nh);
    view_box.set((nx, ny, nw, nh));
  };
  let zoom_in = move |_| zoom_by(0.7);
  let zoom_out = move |_| zoom_by(1.0 / 0.7);
  let reset_view = move |_| view_box.set((0.0, 0.0, MAP_W, MAP_H));

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
    view_box.set((
      (wx - px * nw).clamp(0.0, MAP_W - nw),
      (wy - py * nh).clamp(0.0, MAP_H - nh),
      nw,
      nh,
    ));
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
        (sx0 - dx).clamp(0.0, MAP_W - w),
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
        let nx = (x + (w - nw) / 2.0).clamp(0.0, MAP_W - nw);
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

  // 悬停显示经纬度（桌面端；拖拽/捏合时隐藏）
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
    let lon = svg_x / K - 180.0;
    let lat = 90.0 - svg_y / K;
    hover.set(Some((css_x, css_y, lon, lat)));
  };
  let on_mouse_leave = move |_: web_sys::MouseEvent| hover.set(None);

  // ---- 静态图层 ----
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

  let graticule_v = (0..=18)
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
  let graticule_h = (0..=18)
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

  // 国家/地区名称标注（缩放分级：放大后显示，避免全图拥挤）。
  let country_views = COUNTRY_LABELS
    .iter()
    .map(|&(name, lon, lat)| {
      let (x, y) = project(lon, lat);
      view! {
        <text x=x.to_string() y=y.to_string() text-anchor="middle" class="fill-foreground/40" font-size="8">{name}</text>
      }
    })
    .collect_view();

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"灰线地图"</div>
            <div class="text-xs text-muted-foreground">"实时晨昏圈 · 低频 DX 灰线窗口"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <section class="rounded-xl border bg-card p-4">
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
              aria-label="灰线地图（滚轮缩放、拖拽平移、双指缩放、双击复位）"
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
                // 海洋纵向渐变（模拟光照，随主题 muted 色）
                <linearGradient id="ocean-grad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="var(--muted)" stop-opacity="0.5" />
                  <stop offset="50%" stop-color="var(--muted)" stop-opacity="0.28" />
                  <stop offset="100%" stop-color="var(--muted)" stop-opacity="0.45" />
                </linearGradient>
                // 太阳直射点光晕
                <radialGradient id="sun-glow">
                  <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.8" />
                  <stop offset="100%" stop-color="#f59e0b" stop-opacity="0" />
                </radialGradient>
              </defs>
              // L0 海洋
              <rect x="0" y="0" width="720" height="360" fill="url(#ocean-grad)" />
              // L1 陆地
              <g shape-rendering="geometricPrecision">{land_views}</g>
              // L2 经纬网
              {graticule_v}
              {graticule_h}
              <line x1="0" y1="180" x2="720" y2="180" class="stroke-foreground/25" stroke-width="1" stroke-dasharray="4 4" vector-effect="non-scaling-stroke" />
              <line x1="360" y1="0" x2="360" y2="360" class="stroke-foreground/25" stroke-width="1" stroke-dasharray="4 4" vector-effect="non-scaling-stroke" />
              // 夜半球遮罩（晨昏圈夜侧，随 UTC 实时移动）
              {move || {
                let (doy, hour) = sun.get();
                let decl = solar_declination(doy);
                let sslon = subsolar_longitude(hour);
                view! {
                  <path d=night_path(decl, sslon) fill="#020617" opacity="0.35" class="animate-in fade-in duration-500" />
                }
              }}
              // 刻度与名称标注
              {tick_views}
              {tick_lat_views}
              {name_views}
              // 国家/地区名称（放大后显示）
              <g
                class="fill-foreground/40"
                style=move || {
                  if zoom.get() >= 2.0 { "opacity:0.9" } else { "opacity:0" }
                }
              >
                {country_views}
              </g>
              // 晨昏圈 + 太阳直射点（随 UTC 实时移动）
              {move || {
                let (doy, hour) = sun.get();
                let decl = solar_declination(doy);
                let sslon = subsolar_longitude(hour);
                // 太阳直射点（正午）与反日点（午夜）
                let (sx, sy) = project(sslon, decl);
                let (ax, ay) = project(
                  if sslon >= 0.0 { sslon - 180.0 } else { sslon + 180.0 },
                  -decl,
                );
                view! {
                  <g class="animate-in fade-in duration-700" style="animation-delay: 0.1s">
                  // 晨昏圈：多层带模拟软边过渡
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
                  // 太阳直射点：光晕（呼吸）+ 实心
                  <circle cx=sx.to_string() cy=sy.to_string() r="16" fill="url(#sun-glow)" class="animate-pulse" />
                  <circle cx=sx.to_string() cy=sy.to_string() r="4" fill="#f59e0b" opacity="0.9" />
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
              }}
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

          <div class="mt-3 flex flex-col gap-2">
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span class="font-mono text-sm font-semibold tabular-nums">{move || clock.get()}</span>
              {move || {
                let off = time_offset.get();
                if off.abs() < 1e-9 {
                  view! { <span class="text-muted-foreground">{"当前时间".to_string()}</span> }
                } else {
                  view! {
                    <span class="rounded bg-amber-500/15 px-1.5 py-0.5 font-medium text-amber-600">
                      {format!("偏移 {}{:.1}h", if off > 0.0 { "+" } else { "-" }, off.abs())}
                    </span>
                  }
                }
              }}
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-muted-foreground">"-12h"</span>
              <input
                type="range"
                min="-12"
                max="12"
                step="0.5"
                prop:value=move || time_offset.get().to_string()
                on:input=move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<f64>() {
                    time_offset.set(v);
                  }
                }
                class="h-1.5 flex-1 accent-primary"
                aria-label="时间偏移（小时）"
              />
              <span class="text-xs text-muted-foreground">"+12h"</span>
            </div>
            <div class="flex flex-wrap gap-1">
              {[
                ("-6h", -6.0),
                ("-1h", -1.0),
                ("现在", 0.0),
                ("+1h", 1.0),
                ("+6h", 6.0),
              ]
              .into_iter()
              .map(|(label, h)| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| time_offset.set(h)
                    class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
                  >
                    {label}
                  </button>
                }
              })
              .collect_view()}
            </div>
          </div>

          <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span class="flex items-center gap-1.5">
              <span class="inline-block h-3 w-3 rounded-sm bg-amber-500/40"></span>
              <span>"晨昏圈"</span>
              <span class="ml-1 inline-block h-2.5 w-2.5 rounded-full bg-amber-500"></span>
              <span>"太阳直射点"</span>
            </span>
            <span class="text-muted-foreground">"滚轮/双指缩放 · 拖拽平移 · 双击复位 · 悬停经纬度"</span>
          </div>
          <p class="mt-3 text-xs text-muted-foreground">
            "黄色带为晨昏圈（日出/日落分界），随 UTC 时间实时移动；实心点为太阳直射点、空心为反日点。两端处于灰线的路径，常是 160m / 80m 低频 DX 的黄金窗口。"
          </p>
        </section>
      </div>
    </div>
  }
}
