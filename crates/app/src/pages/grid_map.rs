//! 网格地图：全球已通联 Maidenhead 网格可视化，并支持输入网格码查询具体位置。
//!
//! 「已通联」按统计口径算（与日志页、`/stats`、`/dxcc-map`、`/report` 共用一份持久化值）：
//! 在一处选了野外台站，这里也只画那个台站的网格；「本台」网格也取口径里的台站。

use ham_web_core::grid::{field_index, lat_lon_from_grid};
use ham_web_core::log_stats::StatsScope;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf, tp};
use crate::pages::log::{
  GridMap, StatsScopeSelect, scoped_log_entries, use_log_store, use_stats_scope,
};
use crate::ui::{Field, Input};
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
  let scope = use_stats_scope();
  let scoped = scoped_log_entries(scope);

  // 已通联网格去重列表（响应式，跟随日志、台站档案与口径联动）。
  let grids = Memo::new(move |_| {
    let mut grids: Vec<String> = scoped
      .get()
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
  // 反向地理编码的请求代际：每次输入变化自增，响应回来时对不上就丢弃，
  // 否则「OM89」的慢响应会晚于「OM89EW」的响应到达，把新输入的结果覆盖成旧地点。
  let geocode_gen = StoredValue::new(0u32);
  Effect::new(move |_| {
    let q = query.get().trim().to_ascii_uppercase();
    match lat_lon_from_grid(&q) {
      Some((lat, lon)) => {
        geocode.set(None);
        geocode_gen.update_value(|g| *g = g.wrapping_add(1));
        let my = geocode_gen.get_value();
        spawn_local(async move {
          let Ok(g) =
            data::fetch_external_json::<Geocode>(&format!("/api/geocode?lat={lat}&lon={lon}"))
              .await
          else {
            return;
          };
          // 代际变了（用户又改了输入）或已离开页面（槽位释放）：这次结果作废。
          if geocode_gen.try_get_value() != Some(my) {
            return;
          }
          geocode.set(Some(g));
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
            <span class="border px-3 py-1 text-xs text-muted-foreground">
              {move || tp("radio.grids-worked", count.get(), &[&count.get().to_string()])}
            </span>
          }
        })
      />
      <PageContainer>
        <section class="pxl-card border bg-card p-4">
          <h2 class="mb-3 text-sm font-semibold">{move || t("radio.grid-lookup")}</h2>
          <div class="grid gap-3 sm:grid-cols-2">
            <Field
              label=Signal::derive(move || t("radio.enter-a-maidenhead-grid"))
              r#for="grid-lookup-input"
            >
              <Input
                id="grid-lookup-input"
                value=query
                on_change=Callback::new(move |v: String| query.set(v.to_ascii_uppercase()))
                placeholder=Signal::derive(move || t("radio.e-g-om89ew"))
                maxlength=Signal::derive(|| "6".to_owned())
                class="font-mono uppercase"
              />
            </Field>
            <div class="flex items-center bg-muted/40 px-3 py-2 text-sm tabular-nums">
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
              "radio.centre-lat-lon-with-place",
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

        <section class="pxl-card border bg-card p-4">
          <div class="mb-3">
            <StatsScopeSelect scope=scope count=Signal::derive(move || scoped.with(Vec::len)) />
          </div>
          {move || {
            let entries = scoped.get();
            // 「本台」：口径是某个台站时用它的网格，否则用当前台站（与日志页统计面板一致）。
            let station_grid = match StatsScope::from_key(&scope.get()).station {
              Some(id) => store.station.with(|b| b.resolve(id).gridsquare.clone()),
              None => store.active_station().gridsquare,
            };
            view! {
              <GridMap entries=entries station_grid=station_grid />
            }
          }}
          <p class="mt-3 text-xs text-muted-foreground">
            {move || if count.get() == 0 {
              t("radio.no-grid-records-in")
            } else {
              t("radio.click-a-field-large")
            }}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}
