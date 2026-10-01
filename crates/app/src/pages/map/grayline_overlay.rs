use ham_web_core::astro::{day_of_year, solar_declination, subsolar_longitude, terminator_lat};
use leptos::prelude::*;
use wasm_bindgen::JsValue;

use super::projection::project;

/// 构建灰线带闭合路径（晨昏圈 ± `half_band` 度）。
fn band_path(decl: f64, subsolar_lon: f64, half_band: f64) -> String {
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
fn night_path(decl: f64, sslon: f64) -> String {
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

// ---------------------------------------------------------------------------
// ## 灰线叠加层
// ---------------------------------------------------------------------------

/// 晨昏圈叠加：夜半球遮罩 + 灰线带 + 太阳直射点/反日点。
///
/// `now_ms` 为当前展示时刻（Unix 毫秒），由调用方（灰线页含时间偏移、网格地图含开关）驱动。
#[component]
pub fn GraylineOverlay(#[prop(into)] now_ms: Signal<f64>) -> impl IntoView {
  let sun = Memo::new(move |_| {
    let date = js_sys::Date::new(&JsValue::from_f64(now_ms.get()));
    let doy = day_of_year(
      date.get_utc_full_year(),
      date.get_utc_month() + 1,
      date.get_utc_date(),
    );
    let hour = date.get_utc_hours() as f64 + date.get_utc_minutes() as f64 / 60.0;
    let decl = solar_declination(doy);
    let sslon = subsolar_longitude(hour);
    (decl, sslon)
  });

  view! {
    <g>
      {move || {
        let (decl, sslon) = sun.get();
        let (sx, sy) = project(sslon, decl);
        let (ax, ay) = project(
          if sslon >= 0.0 { sslon - 180.0 } else { sslon + 180.0 },
          -decl,
        );
        view! {
          // 夜半球遮罩
          <path d=night_path(decl, sslon) fill="#020617" opacity="0.28" />
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
        }
      }}
    </g>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn band_path_is_closed() {
    let d = band_path(20.0, 0.0, 6.0);
    assert!(d.starts_with('M'));
    assert!(d.ends_with('Z'));
    assert!(d.contains('L'));
  }
}
