use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::data;
use crate::ui::{Size, Variant, button_class, input_class};

use super::CELL;

/// 一条过境记录。
#[derive(Deserialize, Clone)]
struct PassItem {
  name: String,
  aos: i64,
  los: i64,
  max_elev: f64,
  max_elev_time: i64,
  azimuth: f64,
}

/// `/api/passes` 返回结构。
#[derive(Deserialize)]
struct PassesApi {
  passes: Vec<PassItem>,
}

/// Unix 秒 → 本地时间 `MM-DD HH:MM`。
fn fmt_pass_time(unix: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(unix as f64 * 1000.0));
  let mo = d.get_month() as i32 + 1;
  let day = d.get_date() as i32;
  let h = d.get_hours() as i32;
  let mi = d.get_minutes() as i32;
  format!("{mo:02}-{day:02} {h:02}:{mi:02}")
}

/// 过境预报：输入经纬度，查询未来 24 小时业余卫星过境。
#[component]
pub(super) fn PassPredictor() -> impl IntoView {
  let lat = RwSignal::new(39.9);
  let lon = RwSignal::new(116.4);
  let min_elev = RwSignal::new(10.0);
  let passes = RwSignal::new(Vec::<PassItem>::new());
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let queried = RwSignal::new(false);

  let fetch = move || {
    loading.set(true);
    failed.set(false);
    let (la, lo, me) = (lat.get(), lon.get(), min_elev.get());
    spawn_local(async move {
      let url = format!("/api/passes?lat={la}&lon={lo}&min_elev={me}");
      match data::fetch_external_json::<PassesApi>(&url).await {
        Ok(api) => {
          passes.set(api.passes);
          queried.set(true);
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"过境预报"</h2>
      <div class="space-y-3 p-4">
        <div class="grid gap-3 sm:grid-cols-3">
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"纬度（°）"</span>
            <input
              type="number"
              step="0.0001"
              prop:value=move || lat.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  lat.set(v);
                }
              }
              class=input_class("")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"经度（°）"</span>
            <input
              type="number"
              step="0.0001"
              prop:value=move || lon.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  lon.set(v);
                }
              }
              class=input_class("")
            />
          </label>
          <label class="flex flex-col gap-1.5 text-sm">
            <span class="text-xs text-muted-foreground">"最小仰角（°）"</span>
            <input
              type="number"
              step="1"
              prop:value=move || min_elev.get().to_string()
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<f64>() {
                  min_elev.set(v.clamp(0.0, 90.0));
                }
              }
              class=input_class("")
            />
          </label>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "")
            on:click=move |_| fetch()
          >
            "查询过境"
          </button>
          <span class="text-xs text-muted-foreground">
            "数据来自 Celestrak / SGP4，展示未来 24 小时过境（本地时间）。"
          </span>
        </div>

        {move || {
          if loading.get() {
            return view! {
              <div class="rounded-lg bg-muted/40 px-3 py-8 text-center text-sm text-muted-foreground">
                "正在计算过境预报…"
              </div>
            }
            .into_any();
          }
          if failed.get() {
            return view! {
              <div class="rounded-lg bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                "过境数据暂不可用（可能因网络受限），请稍后重试。"
              </div>
            }
            .into_any();
          }
          if !queried.get() {
            return view! {
              <div class="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                "输入经纬度后点击「查询过境」，默认位置为北京。"
              </div>
            }
            .into_any();
          }
          let items = passes.get();
          if items.is_empty() {
            return view! {
              <div class="rounded-lg bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                "未来 24 小时无符合条件的过境，可降低最小仰角后重试。"
              </div>
            }
            .into_any();
          }
          view! {
            <div class="overflow-x-auto">
              <table class="w-full min-w-[560px] border-collapse text-sm">
                <thead class="bg-muted/60 text-xs">
                  <tr>
                    <th class=CELL>"卫星"</th>
                    <th class=CELL>"开始（AOS）"</th>
                    <th class=CELL>"最大仰角"</th>
                    <th class=CELL>"方位角"</th>
                    <th class=CELL>"时长"</th>
                  </tr>
                </thead>
                <tbody>
                  {items
                    .into_iter()
                    .map(|p| {
                      let dur_min = ((p.los - p.aos) as f64 / 60.0).round() as i64;
                      let name = p.name.clone();
                      let max_t = p.max_elev_time;
                      view! {
                        <tr class="border-t transition-colors hover:bg-muted/40">
                          <td class=format!("{CELL} whitespace-nowrap")>
                            <div class="font-medium">{name}</div>
                            <div
                              class="font-mono text-xs text-muted-foreground"
                              title=format!("最大仰角时刻 {}", fmt_pass_time(max_t))
                            >
                              {fmt_pass_time(p.aos)}
                            </div>
                          </td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{fmt_pass_time(p.aos)}</td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}°", p.max_elev)}</td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}°", p.azimuth)}</td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{dur_min} 分钟")}</td>
                        </tr>
                      }
                    })
                    .collect_view()}
                </tbody>
              </table>
            </div>
          }
          .into_any()
        }}
      </div>
    </section>
  }
}
