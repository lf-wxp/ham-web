use ham_web_core::grid::lat_lon_from_grid;
use ham_web_core::sat_watch::{LEAD_CHOICES, Pass, SatWatch, compass, upcoming};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;
use web_sys::{Notification, NotificationPermission};

use crate::pages::log::use_log_store;
use crate::sat_alert;
use crate::ui::{Size, Variant, button_class, input_class};

use super::CELL;
use crate::i18n::{t, tf};

/// Unix 秒 → 本地时间 `MM-DD HH:MM`。
fn fmt_pass_time(unix: i64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(unix as f64 * 1000.0));
  let mo = d.get_month() as i32 + 1;
  let day = d.get_date() as i32;
  let h = d.get_hours() as i32;
  let mi = d.get_minutes() as i32;
  format!("{mo:02}-{day:02} {h:02}:{mi:02}")
}

fn now_secs() -> i64 {
  (js_sys::Date::now() / 1000.0) as i64
}

fn permission_label() -> String {
  match Notification::permission() {
    NotificationPermission::Granted => t("已允许"),
    NotificationPermission::Denied => t("已被浏览器拒绝，请在站点设置中允许通知"),
    _ => t("尚未授权"),
  }
}

/// 过境预报：查询未来 24 小时业余卫星过境，可收藏卫星并在过境前提醒。
#[component]
pub(super) fn PassPredictor() -> impl IntoView {
  let watch = RwSignal::new(sat_alert::load());
  let update = move |f: &dyn Fn(&mut SatWatch)| {
    watch.update(|w| f(w));
    sat_alert::save(&watch.get_untracked());
  };
  let passes = RwSignal::new(Vec::<Pass>::new());
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let queried = RwSignal::new(false);
  let only_fav = RwSignal::new(false);
  let permission = RwSignal::new(permission_label());
  let station_grid = use_log_store()
    .station
    .with_untracked(|s| s.gridsquare.clone());
  let station_pos = lat_lon_from_grid(&station_grid);

  let fetch = move || {
    loading.set(true);
    failed.set(false);
    let w = watch.get_untracked();
    spawn_local(async move {
      match sat_alert::fetch_passes(&w, "amateur").await {
        Ok(list) => {
          passes.set(list);
          queried.set(true);
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };
  if watch.with_untracked(|w| !w.favorites.is_empty()) {
    fetch();
  }

  let toggle_alerts = move |on: bool| {
    if on
      && matches!(Notification::permission(), NotificationPermission::Default)
      && let Ok(p) = Notification::request_permission()
    {
      spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(p).await;
        permission.set(permission_label());
      });
    }
    update(&|w| w.alerts = on);
  };

  let toggle_apt = move |on: bool| {
    if on
      && matches!(Notification::permission(), NotificationPermission::Default)
      && let Ok(p) = Notification::request_permission()
    {
      spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(p).await;
        permission.set(permission_label());
      });
    }
    update(&|w| w.apt_alert = on);
  };

  let number_input = move |label: &'static str,
                           step: &'static str,
                           get: fn(&SatWatch) -> f64,
                           set: fn(&mut SatWatch, f64)| {
    view! {
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{label}</span>
        <input
          type="number"
          step=step
          prop:value=move || watch.with(get).to_string()
          on:change=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<f64>() {
              update(&|w| set(w, v));
            }
          }
          class=input_class("")
        />
      </label>
    }
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("过境预报与提醒")}</h2>
      <div class="space-y-4 p-4">
        <div class="grid gap-3 sm:grid-cols-3">
          {number_input("纬度（°）", "0.0001", |w| w.lat, |w, v| w.lat = v.clamp(-90.0, 90.0))}
          {number_input("经度（°）", "0.0001", |w| w.lon, |w, v| w.lon = v.clamp(-180.0, 180.0))}
          {number_input("最小仰角（°）", "1", |w| w.min_elev, |w, v| w.min_elev = v.clamp(0.0, 90.0))}
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "")
            on:click=move |_| fetch()
          >
            {move || t("查询过境")}
          </button>
          {station_pos.map(|(la, lo)| {
            view! {
              <button
                type="button"
                class=button_class(Variant::Outline, Size::Default, "")
                on:click=move |_| update(&|w| {
                  w.lat = (la * 100.0).round() / 100.0;
                  w.lon = (lo * 100.0).round() / 100.0;
                })
              >
                {tf("用本台网格 {}", &[&(station_grid).to_string()])}
              </button>
            }
          })}
          <span class="text-xs text-muted-foreground">
            {move || t("数据来自 Celestrak / SGP4，展示未来 24 小时过境（本地时间）。")}
          </span>
        </div>

        <div class="rounded-lg border bg-muted/30 p-3">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
            <label class="inline-flex cursor-pointer items-center gap-2 font-medium">
              <input
                type="checkbox"
                class="size-4 accent-primary"
                prop:checked=move || watch.with(|w| w.alerts)
                on:change=move |e| toggle_alerts(event_target_checked(&e))
              />
              {move || t("收藏卫星过境前通知我")}
            </label>
            <label class="inline-flex cursor-pointer items-center gap-2 font-medium">
              <input
                type="checkbox"
                class="size-4 accent-primary"
                prop:checked=move || watch.with(|w| w.apt_alert)
                on:change=move |e| toggle_apt(event_target_checked(&e))
              />
              {move || t("NOAA 气象卫星过境前提醒我录制 APT")}
            </label>
            <label class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
              {move || t("提前")}
              <select
                class="rounded border bg-background px-1.5 py-0.5"
                prop:value=move || watch.with(|w| w.lead_min.to_string())
                on:change=move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<u32>() {
                    update(&|w| w.lead_min = v);
                  }
                }
              >
                {LEAD_CHOICES.iter().map(|m| view! { <option value=m.to_string()>{tf("{} 分钟", &[&(m).to_string()])}</option> }).collect_view()}
              </select>
            </label>
            <span class="text-xs text-muted-foreground">{move || tf("通知权限：{}", &[&(permission.get()).to_string()])}</span>
          </div>
          <p class="mt-2 text-xs text-muted-foreground">
            {move || t("点击下表中的 ☆ 收藏卫星。提醒在本应用打开期间生效（任意页面，可在后台标签页），关闭浏览器后不会提醒。")}
          </p>
          {move || {
            let w = watch.get();
            if w.favorites.is_empty() {
              return None;
            }
            let list = passes.get();
            let next = upcoming(&w, &list, now_secs());
            let names = w.favorites.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join("、");
            Some(view! {
              <div class="mt-3 space-y-1.5 text-sm">
                <div class="text-xs text-muted-foreground">{tf("已收藏：{}", &[&(names).to_string()])}</div>
                {if next.is_empty() {
                  view! { <div class="text-xs text-muted-foreground">{move || t("查询后在这里显示收藏卫星的下一次过境。")}</div> }.into_any()
                } else {
                  next.into_iter().take(3).map(|p| view! {
                    <div class="flex flex-wrap items-baseline gap-x-2">
                      <span class="font-medium">{p.name.clone()}</span>
                      <span class="font-mono tabular-nums">{fmt_pass_time(p.aos)}</span>
                      <span class="text-xs text-muted-foreground">
                        {tf("最高 {}° · {}方入境 · {} 分钟", &[&format!("{:.0}", p.max_elev), (compass(p.azimuth)), &((p.los - p.aos + 59) / 60).to_string()])}
                      </span>
                    </div>
                  }).collect_view().into_any()
                }}
              </div>
            })
          }}
        </div>

        {move || {
          if loading.get() {
            return view! {
              <div class="rounded-lg bg-muted/40 px-3 py-8 text-center text-sm text-muted-foreground">
                {move || t("正在计算过境预报…")}
              </div>
            }
            .into_any();
          }
          if failed.get() {
            return view! {
              <div class="rounded-lg bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                {move || t("过境数据暂不可用（可能因网络受限），请稍后重试。")}
              </div>
            }
            .into_any();
          }
          if !queried.get() {
            return view! {
              <div class="rounded-lg border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                {move || t("输入经纬度后点击「查询过境」，默认位置为北京。")}
              </div>
            }
            .into_any();
          }
          let w = watch.get();
          let items: Vec<Pass> = passes
            .get()
            .into_iter()
            .filter(|p| !only_fav.get() || w.is_favorite(p.norad))
            .collect();
          view! {
            <div class="space-y-2">
              <label class="inline-flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
                <input
                  type="checkbox"
                  class="size-4 accent-primary"
                  prop:checked=move || only_fav.get()
                  on:change=move |e| only_fav.set(event_target_checked(&e))
                />
                {tf("只看收藏（共 {} 次过境）", &[&(items.len()).to_string()])}
              </label>
              {if items.is_empty() {
                view! {
                  <div class="rounded-lg bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                    {move || t("没有符合条件的过境，可降低最小仰角或取消「只看收藏」。")}
                  </div>
                }.into_any()
              } else {
                view! {
                  <div class="max-h-[32rem] overflow-auto">
                    <table class="w-full min-w-[560px] border-collapse text-sm">
                      <thead class="sticky top-0 bg-muted text-xs">
                        <tr>
                          <th class=CELL><span class="sr-only">{move || t("收藏")}</span></th>
                          <th class=CELL>{move || t("卫星")}</th>
                          <th class=CELL>{move || t("开始（AOS）")}</th>
                          <th class=CELL>{move || t("最大仰角")}</th>
                          <th class=CELL>{move || t("入境方位")}</th>
                          <th class=CELL>{move || t("时长")}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {items
                          .into_iter()
                          .map(|p| {
                            let dur_min = ((p.los - p.aos) as f64 / 60.0).round() as i64;
                            let norad = p.norad;
                            let name = p.name.clone();
                            let fav = move || watch.with(|w| w.is_favorite(norad));
                            view! {
                              <tr class="border-t transition-colors hover:bg-muted/40">
                                <td class=format!("{CELL} w-10 text-center")>
                                  <button
                                    type="button"
                                    class=move || if fav() { "text-amber-500" } else { "text-muted-foreground hover:text-amber-500" }
                                    aria-pressed=move || fav().to_string()
                                    aria-label=tf("收藏 {}", &[&(name).to_string()])
                                    on:click={
                                      let name = name.clone();
                                      move |_| update(&|w| {
                                        w.toggle_favorite(norad, &name);
                                      })
                                    }
                                  >
                                    {move || if fav() { "★" } else { "☆" }}
                                  </button>
                                </td>
                                <td class=format!("{CELL} whitespace-nowrap font-medium")>{p.name.clone()}</td>
                                <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")
                                  title=tf("最大仰角时刻 {}", &[&(fmt_pass_time(p.max_elev_time)).to_string()])
                                >
                                  {fmt_pass_time(p.aos)}
                                </td>
                                <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}°", p.max_elev)}</td>
                                <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}° {}", p.azimuth, compass(p.azimuth))}</td>
                                <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{tf("{} 分钟", &[&(dur_min).to_string()])}</td>
                              </tr>
                            }
                          })
                          .collect_view()}
                      </tbody>
                    </table>
                  </div>
                }.into_any()
              }}
            </div>
          }
          .into_any()
        }}
      </div>
    </section>
  }
}
