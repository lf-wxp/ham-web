use leptos::prelude::*;
use leptos::svg;

use super::projection::{K, MAP_H, MAP_W, WORLD_OFFSETS, polygon_points, project, simplify};
use super::world_data::{CONTINENTS, COUNTRY_LABELS};
use crate::i18n::t;
use crate::ui::TextValue;

/// 两个活动指针的欧氏距离（像素），用于双指捏合缩放。
fn ptr_dist(a: &(i32, f64, f64), b: &(i32, f64, f64)) -> f64 {
  let dx = a.1 - b.1;
  let dy = a.2 - b.2;
  (dx * dx + dy * dy).sqrt()
}

/// 世界地图底图 + 交互：等距圆柱投影，大陆轮廓 + 经纬网 + 缩放/平移/悬停。
///
/// - 等距圆柱投影（`project`）下 Maidenhead 网格退化为轴对齐矩形，几何计算与命中检测简单；
/// - 反子午线环绕：`viewBox.x` 允许越过 ±180°，世界内容在 x 方向平铺三份，全球无缝横拖；
/// - 叠加信息层由 `children` 注入（如 [`GraylineOverlay`] 或网格热力），随底图一同平铺。
#[component]
pub fn MapView(
  #[prop(optional, into)] aria_label: TextValue,
  /// 外部触发定位：(纬度, 经度)，设为 `Some` 时视图居中并缩放到 square 级别。
  #[prop(optional, into)]
  focus: Signal<Option<(f64, f64)>>,
  /// 可选：把当前缩放级别同步给调用方（用于按 zoom 切换信息层粒度）。
  #[prop(optional)]
  zoom_signal: Option<RwSignal<f64>>,
  /// 可选：把悬停处经纬度 `(纬度, 经度)` 同步给调用方（用于悬停联动）。
  #[prop(optional)]
  hover_signal: Option<RwSignal<Option<(f64, f64)>>>,
  /// 可选：把点击处经纬度 `(纬度, 经度)` 同步给调用方（用于点击反查网格）。
  #[prop(optional)]
  click_signal: Option<RwSignal<Option<(f64, f64)>>>,
  children: ChildrenFn,
) -> impl IntoView {
  // 空标签一律回退到「可滚动」提示。取 `TextValue` 而不是 `String`：调用点可能传
  // `Signal::derive(move || t(…))`，静态化之后切语言就不会更新了。
  let label = move || {
    if aria_label.is_empty() {
      t("log.world-map-scroll-to")
    } else {
      aria_label.get()
    }
  };

  // ---- 视图状态（平移/缩放，x 允许环绕）----
  let view_box = RwSignal::new((0.0f64, 0.0f64, MAP_W, MAP_H));
  let zoom = Signal::derive(move || MAP_W / view_box.get().2);
  // 把缩放级别同步给调用方（如网格地图按 zoom 切换 field/square 层级）。
  if let Some(zs) = zoom_signal {
    Effect::new(move |_| {
      zs.set(zoom.get());
    });
  }
  let dragging = RwSignal::new(false);
  let drag_start = StoredValue::new((0.0f64, 0.0f64, 0.0f64, 0.0f64));
  let svg_ref = NodeRef::<svg::Svg>::new();
  let active_pointers = RwSignal::new(Vec::<(i32, f64, f64)>::new());
  let pinch_state = StoredValue::new((0.0f64, (0.0f64, 0.0f64, MAP_W, MAP_H)));
  let hover = RwSignal::new(None::<(f64, f64, f64, f64)>);
  // 点击反查：记录 pointerdown 时的客户区坐标，用于区分「点击」与「拖拽」。
  let click_anchor = StoredValue::new((0.0f64, 0.0f64));

  // 外部定位（搜索定位等）：聚焦到指定经纬度并缩放到 square 级别。
  Effect::new(move |_| {
    let Some((lat, lon)) = focus.get() else {
      return;
    };
    let (px, py) = project(lon, lat);
    let w = MAP_W / 8.0;
    let h = MAP_H / 8.0;
    view_box.set((
      (px - w / 2.0).rem_euclid(MAP_W),
      (py - h / 2.0).clamp(0.0, MAP_H - h),
      w,
      h,
    ));
  });

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
    click_anchor.set_value((f64::from(ev.client_x()), f64::from(ev.client_y())));
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
    if let Some(hs) = hover_signal {
      hs.set(Some((lat, lon)));
    }
  };
  let on_mouse_leave = move |_: web_sys::MouseEvent| {
    hover.set(None);
    if let Some(hs) = hover_signal {
      hs.set(None);
    }
  };

  // 点击反查：位移小于阈值（视为点击而非拖拽）时，把经纬度同步给调用方。
  let on_click = move |ev: web_sys::MouseEvent| {
    let (ax, ay) = click_anchor.get_value();
    let dx = f64::from(ev.client_x()) - ax;
    let dy = f64::from(ev.client_y()) - ay;
    if dx * dx + dy * dy > 16.0 {
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
    let lon = (svg_x / K).rem_euclid(360.0) - 180.0;
    let lat = 90.0 - svg_y / K;
    if let Some(cs) = click_signal {
      cs.set(Some((lat, lon)));
    }
  };

  // ---- 静态图层（构建一次，随环绕平铺克隆）----
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
      <text x=x.to_string() y=y.to_string() text-anchor="middle" class=cls font-size=size>{move || t(name)}</text>
    }
  })
  .collect_view();

  let country_views = COUNTRY_LABELS
    .iter()
    .map(|&(name, lon, lat)| {
      let (x, y) = project(lon, lat);
      view! {
        <text
          x=x.to_string()
          y=y.to_string()
          text-anchor="middle"
          class="fill-foreground/40"
          font-size="8"
        >{move || t(name)}</text>
      }
    })
    .collect_view();

  // 叠加层（children）为 `Fn`，每个平铺调用一次构建独立信息层，随底图一同平铺。

  view! {
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
        aria-label=label
        preserveAspectRatio="xMidYMid meet"
        on:wheel=on_wheel
        on:pointerdown=on_pointer_down
        on:pointermove=on_pointer_move
        on:pointerup=on_pointer_up
        on:pointerleave=on_pointer_up
        on:pointercancel=on_pointer_up
        on:mousemove=on_mouse_move
        on:mouseleave=on_mouse_leave
        on:click=on_click
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
                {graticule_v.clone()}
                {graticule_h.clone()}
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
                // 信息层（灰线 / 网格热力等）
                {children()}
              </g>
            }
          })
          .collect_view()}
      </svg>

      // 悬停经纬度提示
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

      // 缩放控制
      <div class="absolute right-2 top-2 flex flex-col gap-1">
        <button
          type="button"
          class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
          on:click=zoom_in
          aria-label=move || t("log.zoom-in")
        >
          "+"
        </button>
        <button
          type="button"
          class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
          on:click=zoom_out
          aria-label=move || t("log.zoom-out")
        >
          "−"
        </button>
        <button
          type="button"
          class="flex h-7 w-7 items-center justify-center rounded-md border bg-background text-sm shadow-sm transition-colors hover:bg-accent"
          on:click=reset_view
          aria-label=move || t("log.reset-view")
        >
          "⤢"
        </button>
      </div>
    </div>
  }
}
