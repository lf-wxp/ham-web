//! 灰线地图：实时晨昏圈（日出/日落分界），用于判断低频 DX 的灰线窗口。

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::pages::log::{CONTINENTS, polygon_points, project};
use crate::util::set_title;

/// 一年中的第几天（1–366）。
fn day_of_year(year: u32, month: u32, day: u32) -> u32 {
  let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
  let cum = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
  let mut doy = cum[(month - 1) as usize] + day;
  if leap && month > 2 {
    doy += 1;
  }
  doy
}

/// 太阳赤纬（度），近似公式。
fn solar_declination(day_of_year: u32) -> f64 {
  23.44 * (2.0 * std::f64::consts::PI * (day_of_year as f64 - 81.0) / 365.25).sin()
}

/// 太阳直射点经度（度，东经为正），由 UTC 小时推算。
fn subsolar_longitude(utc_hour: f64) -> f64 {
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
fn band_path(decl: f64, subsolar_lon: f64, half_band: f64) -> String {
  let mut upper: Vec<(f64, f64)> = Vec::new();
  let mut lower: Vec<(f64, f64)> = Vec::new();
  for x in (0..=720).step_by(4) {
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

#[component]
pub fn GraylinePage() -> impl IntoView {
  set_title("灰线地图");

  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  set_interval(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  );

  let sun = Memo::new(move |_| {
    let ms = now.get();
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
          <svg
            viewBox="0 0 720 360"
            class="w-full text-primary"
            role="img"
            aria-label="灰线地图"
          >
            <rect x="0" y="0" width="720" height="360" fill="currentColor" opacity="0.03" />
            {CONTINENTS
              .iter()
              .map(|poly| {
                view! {
                  <polygon points=polygon_points(poly) fill="currentColor" opacity="0.08" />
                }
              })
              .collect_view()}
            <line
              x1="0"
              y1="180"
              x2="720"
              y2="180"
              stroke="currentColor"
              opacity="0.12"
              stroke-dasharray="4 4"
            />
            <line
              x1="360"
              y1="0"
              x2="360"
              y2="360"
              stroke="currentColor"
              opacity="0.12"
              stroke-dasharray="4 4"
            />
            {move || {
              let (doy, hour) = sun.get();
              let decl = solar_declination(doy);
              let sslon = subsolar_longitude(hour);
              view! {
                <path d=band_path(decl, sslon, 6.0) fill="#f59e0b" opacity="0.25" />
                <path
                  d=band_path(decl, sslon, 0.3)
                  fill="none"
                  stroke="#f59e0b"
                  stroke-width="1"
                  opacity="0.7"
                />
              }
            }}
          </svg>
          <p class="mt-3 text-xs text-muted-foreground">
            "黄色带为晨昏圈（日出/日落分界），随 UTC 时间实时移动。两端处于灰线的路径，常是 160m / 80m 低频 DX 的黄金窗口。"
          </p>
        </section>
      </div>
    </div>
  }
}
