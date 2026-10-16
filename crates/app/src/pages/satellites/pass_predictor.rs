use ham_web_core::grid::lat_lon_from_grid;
use ham_web_core::sat_watch::{LEAD_CHOICES, Pass, SatWatch, compass, upcoming};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;
use web_sys::{Notification, NotificationPermission};

use crate::pages::log::use_log_store;
use crate::sat_alert;
use crate::ui::{
  Button, Checkbox, ControlSize, Field, NativeSelect, NumberField, SelectOption, Size, Table,
  TableBody, TableCell, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::notification_permission;

use super::CELL;
use crate::i18n::{t, tf, tp};

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
  match notification_permission() {
    None => t("radio.notifications-unsupported"),
    Some(NotificationPermission::Granted) => t("radio.allowed"),
    Some(NotificationPermission::Denied) => t("radio.blocked-by-the-browser"),
    Some(_) => t("radio.not-authorised-yet"),
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
  let station_grid = use_log_store().active_station().gridsquare;
  let station_pos = lat_lon_from_grid(&station_grid);

  // 查询代际：挂载时的自动查询与用户手动点「查询过境」可能并发，快照又各不相同；
  // 响应回来时对不上代际就丢弃，避免旧坐标的结果晚到后覆盖新查询的结果。
  let fetch_gen = StoredValue::new(0u32);
  let fetch = move || {
    loading.set(true);
    failed.set(false);
    let w = watch.get_untracked();
    fetch_gen.update_value(|g| *g = g.wrapping_add(1));
    let my = fetch_gen.get_value();
    spawn_local(async move {
      let outcome = sat_alert::fetch_passes(&w, "amateur").await;
      if fetch_gen.try_get_value() != Some(my) {
        return;
      }
      match outcome {
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
      && notification_permission() == Some(NotificationPermission::Default)
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
      && notification_permission() == Some(NotificationPermission::Default)
      && let Ok(p) = Notification::request_permission()
    {
      spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(p).await;
        permission.set(permission_label());
      });
    }
    update(&|w| w.apt_alert = on);
  };

  // 标签收 `Signal` 而不是一次性翻好的 `String`：`Field` 的标签要跟随语言切换。
  let number_input = move |label: Signal<String>,
                           step: &'static str,
                           get: fn(&SatWatch) -> f64,
                           set: fn(&mut SatWatch, f64)| {
    let id = crate::util::unique_id("pass-predictor-num");
    let label_for = id.clone();
    // 调用点传的是字符串形式的步进（"0.0001" / "1"），`NumberField` 要 f64。
    let step_value = step.parse::<f64>().unwrap_or(1.0);
    view! {
      <Field label=label r#for=label_for>
        <NumberField
          id=id
          value=Signal::derive(move || watch.with(get).to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(v) = v.trim().parse::<f64>() {
              update(&|w| set(w, v));
            }
          })
          step=step_value
          controls=false
        />
      </Field>
    }
  };

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.pass-predictions-and-reminders")}</h2>
      <div class="space-y-4 p-4">
        <div class="grid gap-3 sm:grid-cols-3">
          {number_input(Signal::derive(move || t("common.latitude")), "0.0001", |w| w.lat, |w, v| w.lat = v.clamp(-90.0, 90.0))}
          {number_input(Signal::derive(move || t("common.longitude")), "0.0001", |w| w.lon, |w, v| w.lon = v.clamp(-180.0, 180.0))}
          {number_input(Signal::derive(move || t("common.min-elevation")), "1", |w| w.min_elev, |w, v| w.min_elev = v.clamp(0.0, 90.0))}
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <Button
            variant=Variant::Default
            size=Size::Default
            on_click=Callback::new(move |_| fetch())
          >
            {move || t("radio.look-up-passes")}
          </Button>
          {station_pos.map(|(la, lo)| {
            view! {
              <Button
                variant=Variant::Outline
                size=Size::Default
                on_click=Callback::new(move |_| update(&|w| {
                                w.lat = (la * 100.0).round() / 100.0;
                                w.lon = (lo * 100.0).round() / 100.0;
                              }))
              >
                {tf("radio.use-my-grid", &[&(station_grid).to_string()])}
              </Button>
            }
          })}
          <span class="text-xs text-muted-foreground">
            {move || t("radio.data-from-celestrak-sgp4")}
          </span>
        </div>

        <div class="border bg-muted/30 p-3">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
            <label class="inline-flex cursor-pointer items-center gap-2 font-medium">
              <Checkbox
                checked=Signal::derive(move || watch.with(|w| w.alerts))
                on_change=Callback::new(move |on: bool| toggle_alerts(on))
              />
              {move || t("radio.notify-me-before-a")}
            </label>
            <label class="inline-flex cursor-pointer items-center gap-2 font-medium">
              <Checkbox
                checked=Signal::derive(move || watch.with(|w| w.apt_alert))
                on_change=Callback::new(move |on: bool| toggle_apt(on))
              />
              {move || t("radio.remind-me-to-record")}
            </label>
            <label class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
              {move || t("radio.ahead")}
              {{
                let lead_options: Vec<SelectOption> = LEAD_CHOICES
                  .iter()
                  .copied()
                  .map(|m| {
                    SelectOption::new(
                      m.to_string(),
                      Signal::derive(move || tf("radio.min", &[&m.to_string()])),
                    )
                  })
                  .collect();
                view! {
                  <NativeSelect
                    value=Signal::derive(move || watch.with(|w| w.lead_min.to_string()))
                    on_change=Callback::new(move |v: String| {
                      if let Ok(n) = v.parse::<u32>() {
                        update(&|w| w.lead_min = n);
                      }
                    })
                    options=lead_options
                    size=ControlSize::Sm
                    aria_label=Signal::derive(move || t("radio.ahead"))
                    class="w-auto"
                  />
                }
              }}
            </label>
            <span class="text-xs text-muted-foreground">{move || tf("radio.notification-permission", &[&(permission.get()).to_string()])}</span>
          </div>
          <p class="mt-2 text-xs text-muted-foreground">
            {move || t("radio.tap-in-the-table")}
          </p>
          {move || {
            let w = watch.get();
            if w.favorites.is_empty() {
              return None;
            }
            let list = passes.get();
            let next = upcoming(&w, &list, now_secs());
            let names = w.favorites.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", ");
            Some(view! {
              <div class="mt-3 space-y-1.5 text-sm">
                <div class="text-xs text-muted-foreground">{tf("radio.saved", &[&(names).to_string()])}</div>
                {if next.is_empty() {
                  view! { <div class="text-xs text-muted-foreground">{move || t("radio.look-up-a-location")}</div> }.into_any()
                } else {
                  next.into_iter().take(3).map(|p| view! {
                    <div class="flex flex-wrap items-baseline gap-x-2">
                      <span class="font-medium">{p.name.clone()}</span>
                      <span class="font-mono tabular-nums">{fmt_pass_time(p.aos)}</span>
                      <span class="text-xs text-muted-foreground">
                        {tf("radio.max-entering-from-min", &[&format!("{:.0}", p.max_elev), (compass(p.azimuth)), &((p.los - p.aos + 59) / 60).to_string()])}
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
              <div class="bg-muted/40 px-3 py-8 text-center text-sm text-muted-foreground">
                {move || t("radio.calculating-pass-predictions")}
              </div>
            }
            .into_any();
          }
          if failed.get() {
            return view! {
              <div class="bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                {move || t("radio.pass-data-is-unavailable")}
              </div>
            }
            .into_any();
          }
          if !queried.get() {
            return view! {
              <div class="border border-dashed px-3 py-6 text-center text-sm text-muted-foreground">
                {move || t("radio.enter-latitude-and-longitude")}
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
                <Checkbox
                  checked=only_fav
                  on_change=Callback::new(move |on: bool| only_fav.set(on))
                />
                {tp(
                  "radio.favourites-only-passes",
                  items.len() as u32,
                  &[&(items.len()).to_string()],
                )}
              </label>
              {if items.is_empty() {
                view! {
                  <div class="bg-muted/40 px-3 py-6 text-center text-sm text-muted-foreground">
                    {move || t("radio.no-passes-match-the")}
                  </div>
                }.into_any()
              } else {
                view! {
                  <Table class="min-w-[560px] text-sm" wrapper_class="max-h-[32rem]">
                    <TableHead class="sticky top-0 z-10 bg-muted text-xs">
                      <TableRow>
                        <TableHeader class=CELL><span class="sr-only">{move || t("exam.bookmark")}</span></TableHeader>
                        <TableHeader class=CELL>{move || t("radio.satellite")}</TableHeader>
                        <TableHeader class=CELL>{move || t("radio.start-aos")}</TableHeader>
                        <TableHeader class=CELL>{move || t("radio.max-elevation")}</TableHeader>
                        <TableHeader class=CELL>{move || t("radio.entry-bearing")}</TableHeader>
                        <TableHeader class=CELL>{move || t("radio.duration")}</TableHeader>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {items
                        .into_iter()
                        .map(|p| {
                          let dur_min = ((p.los - p.aos) as f64 / 60.0).round() as i64;
                          let norad = p.norad;
                          let name = p.name.clone();
                          let fav = move || watch.with(|w| w.is_favorite(norad));
                          view! {
                            <TableRow>
                              <TableCell class=format!("{CELL} w-10 text-center")>
                                <button
                                  type="button"
                                  class=move || if fav() { "text-amber-500" } else { "text-muted-foreground hover:text-amber-500" }
                                  aria-pressed=move || fav().to_string()
                                  aria-label=tf("radio.save", &[&(name).to_string()])
                                  on:click={
                                    let name = name.clone();
                                    move |_| update(&|w| {
                                      w.toggle_favorite(norad, &name);
                                    })
                                  }
                                >
                                  {move || if fav() { "★" } else { "☆" }}
                                </button>
                              </TableCell>
                              <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>{p.name.clone()}</TableCell>
                              <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")
                                title=tf("radio.max-elevation-at", &[&(fmt_pass_time(p.max_elev_time)).to_string()])
                              >
                                {fmt_pass_time(p.aos)}
                              </TableCell>
                              <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}°", p.max_elev)}</TableCell>
                              <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>{format!("{:.0}° {}", p.azimuth, compass(p.azimuth))}</TableCell>
                              <TableCell class=format!("{CELL} whitespace-nowrap tabular-nums")>{tf("radio.min", &[&(dur_min).to_string()])}</TableCell>
                            </TableRow>
                          }
                        })
                        .collect_view()}
                    </TableBody>
                  </Table>
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
