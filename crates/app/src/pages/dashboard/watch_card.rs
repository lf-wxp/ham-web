//! 现在守哪一段：这个钟点、这张操作证，该守哪几个波段窗口。
//!
//! 窗口的筛选、可靠度与灰线几何全在核心 `ham_web_core::watch_windows`（有单测）：证不允许的窗口不出现；
//! 只有本台网格、对方网格、太阳黑子数都有时才给可靠度并排序，缺任何一样就只列窗口、不编「好 / 差」。
//! K ≥ 4 沿用上面太阳卡的传播等级提示，**不折减**可靠度。
//!
//! 操作证类别与 CAT 频率护栏共用同一份设置（`cat-settings` 的 `class`，设备专属、不同步）。

use std::time::Duration;

use ham_web_core::license_compliance::Class;
use ham_web_core::solar::live_level;
use ham_web_core::watch_windows::{GreyKind, When, watch};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use super::watch_window_row::WatchWindowRow;
use crate::components::cat_control::{load_settings, save_settings};
use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::{ChipToggle, Field, Input, card_title_class};
use crate::util::unique_id;

/// `/api/solar` 里本卡要用的三项。
#[derive(Deserialize, Clone)]
struct SolarApi {
  k_index: Option<i32>,
  solar_flux: Option<i32>,
  sunspots: Option<i32>,
}

/// 此刻的 UTC 日期与小时。
fn now_utc() -> When {
  let d = js_sys::Date::new_0();
  When {
    year: d.get_utc_full_year(),
    month: d.get_utc_month() + 1,
    day: d.get_utc_date(),
    utc_hour: f64::from(d.get_utc_hours()) + f64::from(d.get_utc_minutes()) / 60.0,
  }
}

#[component]
pub(super) fn WatchCard() -> impl IntoView {
  // 传播等级的文字来自核心里的中文原文，要挂上词典拉取，en / es 才不停在中文。
  data::use_knowledge_i18n();
  let store = use_log_store();
  let class = RwSignal::new(load_settings().class);
  let home = RwSignal::new(store.active_station_untracked().gridsquare);
  let target = RwSignal::new(String::new());
  let when = RwSignal::new(now_utc());
  let solar = RwSignal::new(None::<SolarApi>);

  // 时间每 30 秒刷一次（分钟级足够：可靠度按整小时的昼夜因子算，灰线距离每分钟才动 ~28 km）。
  if let Ok(handle) = set_interval_with_handle(move || when.set(now_utc()), Duration::from_secs(30))
  {
    on_cleanup(move || handle.clear());
  }
  let refresh = move || {
    spawn_local(async move {
      if let Ok(api) = data::fetch_external_json::<SolarApi>("/api/solar").await {
        solar.try_set(Some(api));
      }
    });
  };
  refresh();
  if let Ok(handle) = set_interval_with_handle(refresh, Duration::from_secs(60)) {
    on_cleanup(move || handle.clear());
  }

  let set_class = move |c: Class, on: bool| {
    let next = on.then_some(c);
    class.set(next);
    // 只改 `class`：串口参数是同一份设置里别人的字段。
    let mut s = load_settings();
    s.class = next;
    save_settings(&s);
  };

  let ssn = Memo::new(move |_| {
    solar
      .with(|s| s.as_ref().and_then(|s| s.sunspots))
      .map(f64::from)
  });
  let result = Memo::new(move |_| {
    class
      .get()
      .map(|c| watch(c, &home.get(), &target.get(), &when.get(), ssn.get()))
  });
  let level = Memo::new(move |_| {
    solar.with(|s| {
      s.as_ref()
        .and_then(|s| live_level(s.k_index, s.sunspots, s.solar_flux))
    })
  });

  let home_id = unique_id("watch-home");
  let target_id = unique_id("watch-target");

  view! {
    <section class=crate::kit::card_class() data-testid="watch-card">
      <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
        <h2 class=card_title_class("min-w-0 flex-1")>{move || t("radio.watch-title")}</h2>
        <span class="font-mono text-xs tabular-nums text-muted-foreground" data-testid="watch-clock">
          {move || {
            let w = when.get();
            let minutes = (w.utc_hour * 60.0).floor() as u32;
            format!("UTC {:02}:{:02}", minutes / 60, minutes % 60)
          }}
        </span>
      </div>
      <div class="space-y-3 p-4">
        <p class="text-xs text-muted-foreground">{move || t("radio.watch-intro")}</p>
        <div class="flex flex-wrap items-center gap-1.5" role="group" aria-label=move || t("radio.compliance-class-label")>
          <span class="mr-1 text-xs text-muted-foreground">{move || t("radio.compliance-class-label")}</span>
          {[Class::A, Class::B, Class::C]
            .into_iter()
            .map(|c| {
              view! {
                <ChipToggle
                  active=Signal::derive(move || class.get() == Some(c))
                  on_change=Callback::new(move |on: bool| set_class(c, on))
                >
                  {move || tf("learning.class", &[c.label()])}
                </ChipToggle>
              }
            })
            .collect_view()}
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <Field label=Signal::derive(move || t("log.station-grid")) r#for=home_id.clone()>
            <Input
              id=home_id
              class="font-mono uppercase"
              placeholder="OM89"
              value=home
              on_change=Callback::new(move |v: String| home.set(v.to_uppercase()))
            />
          </Field>
          <Field label=Signal::derive(move || t("radio.eme-dx-grid")) r#for=target_id.clone()>
            <Input
              id=target_id
              class="font-mono uppercase"
              placeholder="IO91"
              value=target
              on_change=Callback::new(move |v: String| target.set(v.to_uppercase()))
            />
          </Field>
        </div>

        <p class="text-xs" data-testid="watch-level">
          <span class="text-muted-foreground">{move || t("radio.watch-conditions")}</span>
          " "
          {move || {
            level
              .get()
              .map_or_else(|| "—".to_owned(), |l| data::core_text_tracked(l.label()))
          }}
        </p>
        {move || {
          result
            .get()
            .and_then(|r| r.grey_line)
            .map(|g| {
              let side = match g.kind {
                GreyKind::Dawn => t("radio.watch-dawn"),
                GreyKind::Dusk => t("radio.watch-dusk"),
              };
              view! {
                <p class="text-xs" data-testid="watch-grey">
                  {tf("radio.watch-grey", &[&format!("{:.0}", g.distance_km), &side])}
                  {g.near.then(|| view! {
                    <span class="ml-2 border px-1.5 py-0.5 text-muted-foreground">
                      {t("radio.watch-grey-near")}
                    </span>
                  })}
                </p>
              }
            })
        }}
        {move || {
          result
            .get()
            .and_then(|r| r.path)
            .map(|p| {
              view! {
                <p class="text-xs" data-testid="watch-path">
                  {tf(
                    "radio.watch-path",
                    &[&format!("{:.0}", p.bearing_deg), &format!("{:.0}", p.distance_km)],
                  )}
                </p>
              }
            })
        }}

        {move || match result.get() {
          None => {
            view! { <p class="text-sm text-muted-foreground" data-testid="watch-pick">{t("radio.watch-pick-class")}</p> }
              .into_any()
          }
          Some(r) => {
            let ranked = r.ranked;
            view! {
              <ul class="space-y-1.5" data-testid="watch-windows" data-ranked=ranked.to_string()>
                {r.windows.into_iter().map(|w| view! { <WatchWindowRow w=w /> }).collect_view()}
              </ul>
            }
              .into_any()
          }
        }}

        <p class="text-xs text-muted-foreground">{move || t("radio.watch-note")}</p>
      </div>
    </section>
  }
}
