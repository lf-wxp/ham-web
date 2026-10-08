//! 网格地图：全球已通联 Maidenhead 网格可视化，并支持输入网格码查询具体位置。

use ham_web_core::grid::{field_index, lat_lon_from_grid};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf, tp};
use crate::pages::log::{GridMap, use_log_store};
use crate::ui::Input;
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
  set_title("shell.grid-map");

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
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("shell.grid-map"))
        subtitle=Signal::derive(move || t("radio.maidenhead-grid-locator-lookup"))
        actions=ViewFn::from(move || {
          view! {
            <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
              {move || tp("radio.grids-worked", count.get(), &[&count.get().to_string()])}
            </span>
          }
        })
      />
      <PageContainer>
        <section class="rounded-xl border bg-card p-4">
          <h2 class="mb-3 text-sm font-semibold">{move || t("radio.grid-lookup")}</h2>
          <div class="grid gap-3 sm:grid-cols-2">
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">{move || t("radio.enter-a-maidenhead-grid")}</span>
              <Input
                value=query
                on_change=Callback::new(move |v: String| query.set(v.to_ascii_uppercase()))
                placeholder=Signal::derive(move || t("radio.e-g-om89ew"))
                maxlength=Signal::derive(|| "6".to_owned())
                class="font-mono uppercase"
              />
            </label>
            <div class="flex items-center rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums">
              {move || {
                let q = query.get().trim().to_ascii_uppercase();
                if q.is_empty() {
                  t("radio.enter-a-grid-4")
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
                        tf(
              "radio.centre-lat-lon",
              &[&q, &format!("{lat:.4}"), &format!("{lon:.4}")],
            )
                      } else {
                        tf(
              "radio.centre-lat-lon-2",
              &[&q, &format!("{lat:.4}"), &format!("{lon:.4}"), &loc],
            )
                      }
                    }
                    None => t("radio.invalid-grid-must-be"),
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
              {move || t("radio.no-grid-records-in")}
            } else {
              {move || t("radio.click-a-field-large")}
            }}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}
