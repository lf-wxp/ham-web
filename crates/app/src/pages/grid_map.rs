//! 网格地图：全球已通联 Maidenhead 网格可视化，并支持输入网格码查询具体位置。

use ham_web_core::grid::{field_index, lat_lon_from_grid};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::pages::log::{GridMap, use_log_store};
use crate::util::set_title;

/// 反向地理编码结果（/api/geocode）。
#[derive(Deserialize, Clone)]
struct Geocode {
  #[serde(default)]
  country: String,
  #[serde(default)]
  city: String,
}

#[component]
pub fn GridMapPage() -> impl IntoView {
  set_title("网格地图");

  let store = use_log_store();
  let query = RwSignal::new(String::new());

  // 已通联网格去重列表（响应式，跟随日志 store 联动）。
  let grids = Memo::new(move |_| {
    let mut grids: Vec<String> = store
      .logbook
      .get()
      .entries
      .iter()
      .filter_map(|e| {
        let g = e.gridsquare.trim().to_ascii_uppercase();
        field_index(&g).is_some().then_some(g)
      })
      .collect();
    grids.sort();
    grids.dedup();
    grids
  });
  let count = Memo::new(move |_| grids.get().len());

  let geocode = RwSignal::new(None::<Geocode>);
  Effect::new(move |_| {
    let q = query.get().trim().to_ascii_uppercase();
    match lat_lon_from_grid(&q) {
      Some((lat, lon)) => {
        geocode.set(None);
        spawn_local(async move {
          if let Ok(g) =
            data::fetch_external_json::<Geocode>(&format!("/api/geocode?lat={lat}&lon={lon}")).await
          {
            geocode.set(Some(g));
          }
        });
      }
      None => geocode.set(None),
    }
  });

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"网格地图"</h1>
            <div class="text-xs text-muted-foreground">"Maidenhead 网格定位 · 查询与已通联分布"</div>
          </div>
          <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            {move || format!("已通联 {} 个网格", count.get())}
          </span>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <section class="rounded-xl border bg-card p-4">
          <h2 class="mb-3 text-sm font-semibold">"网格查询"</h2>
          <div class="grid gap-3 sm:grid-cols-2">
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"输入 Maidenhead 网格码"</span>
              <input
                type="text"
                placeholder="如 OM89EW"
                maxlength="6"
                prop:value=move || query.get()
                on:input=move |e| query.set(event_target_value(&e).to_ascii_uppercase())
                class="h-10 rounded-lg border bg-background px-3 font-mono text-sm uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              />
            </label>
            <div class="flex items-center rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
              {move || {
                let q = query.get().trim().to_ascii_uppercase();
                if q.is_empty() {
                  "输入网格码（≥4 位，如 OM89）查询其中心经纬度。".to_owned()
                } else {
                  match lat_lon_from_grid(&q) {
                    Some((lat, lon)) => {
                      let loc = match geocode.get() {
                        Some(g) if !g.country.is_empty() => {
                          if g.city.is_empty() {
                            g.country
                          } else {
                            format!("{} · {}", g.country, g.city)
                          }
                        }
                        _ => String::new(),
                      };
                      if loc.is_empty() {
                        format!("{q} 中心：纬度 {lat:.4}°，经度 {lon:.4}°")
                      } else {
                        format!("{q} 中心：纬度 {lat:.4}°，经度 {lon:.4}°　·　{loc}")
                      }
                    }
                    None => "无效网格码：需 4 或 6 位（如 OM89 / OM89EW）。".to_owned(),
                  }
                }
              }}
            </div>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-4">
          {move || {
            let entries = store.logbook.get().entries;
            let station_grid = store.station.get().gridsquare.clone();
            view! {
              <GridMap entries=entries station_grid=station_grid />
            }
          }}
          <p class="mt-3 text-xs text-muted-foreground">
            {move || if count.get() == 0 {
              "暂无日志网格记录，地图仅展示查询标记；在「通联日志」添加带网格的记录后展示通联分布。"
            } else {
              "点击地图上的 field（大格）展开具体网格列表；色块深浅表示通联密度，红色标记为查询位置。"
            }}
          </p>
        </section>
      </div>
    </div>
  }
}
