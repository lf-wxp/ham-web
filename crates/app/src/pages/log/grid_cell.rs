use ham_web_core::grid::lat_lon_from_grid;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;

/// 反向地理编码结果（/api/geocode）。
#[derive(Deserialize, Clone)]
struct Geocode {
  #[serde(default)]
  country: String,
  #[serde(default)]
  city: String,
}

/// 网格单元格：显示网格码并异步补充国家 / 城市。
#[component]
pub(super) fn GridCell(grid: String) -> impl IntoView {
  let geo = RwSignal::new(None::<Geocode>);
  if let Some((lat, lon)) = lat_lon_from_grid(&grid) {
    spawn_local(async move {
      if let Ok(g) =
        data::fetch_external_json::<Geocode>(&format!("/api/geocode?lat={lat}&lon={lon}")).await
      {
        geo.set(Some(g));
      }
    });
  }

  view! {
    <div>
      <div class="font-mono text-muted-foreground">{grid}</div>
      {move || {
        geo.get().map(|g| {
          let loc = if !g.country.is_empty() && !g.city.is_empty() {
            format!("{} · {}", g.country, g.city)
          } else if !g.country.is_empty() {
            g.country
          } else {
            g.city
          };
          if loc.is_empty() {
            view! { <div></div> }.into_any()
          } else {
            view! { <div class="text-xs text-muted-foreground">{loc}</div> }.into_any()
          }
        })
      }}
    </div>
  }
}
