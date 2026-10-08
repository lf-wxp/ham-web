//! SOTA / POTA 编号查询：按编号查询山峰 / 公园详情，并对照本地日志统计激活次数。

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::i18n::t;
use crate::pages::log::use_log_store;
use crate::ui::{Button, Input, Size, Variant};
use crate::util::alert;

/// SOTA 山峰详情。
#[derive(serde::Deserialize, Clone)]
struct SummitInfo {
  reference: String,
  name: String,
  region: String,
  altitude_m: i32,
  points: i32,
  latitude: f64,
  longitude: f64,
}

/// POTA 公园详情。
#[derive(serde::Deserialize, Clone)]
struct ParkInfo {
  reference: String,
  name: String,
  entity: String,
  grid: String,
  latitude: f64,
  longitude: f64,
}

/// 查询结果。
#[derive(Clone)]
enum LookupResult {
  Summit(SummitInfo),
  Park(ParkInfo),
}

#[component]
pub fn PortableLookup() -> impl IntoView {
  let pota = RwSignal::new(false);
  let code = RwSignal::new(String::new());
  let result = RwSignal::new(None::<LookupResult>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  let store = use_log_store();

  let run = move || {
    let c = code.get().trim().to_uppercase();
    if c.len() < 3 {
      alert("请输入编号，如 G/SP-001 或 K-0001");
      return;
    }
    loading.set(true);
    failed.set(false);
    result.set(None);
    let is_pota = pota.get();
    let url = if is_pota {
      format!("/api/pota?ref={c}")
    } else {
      format!("/api/sota?ref={c}")
    };
    spawn_local(async move {
      if is_pota {
        match data::fetch_external_json::<ParkInfo>(&url).await {
          Ok(p) => result.set(Some(LookupResult::Park(p))),
          Err(_) => failed.set(true),
        }
      } else {
        match data::fetch_external_json::<SummitInfo>(&url).await {
          Ok(s) => result.set(Some(LookupResult::Summit(s))),
          Err(_) => failed.set(true),
        }
      }
      loading.set(false);
    });
  };

  let local_count = move |reference: String| {
    store.logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| {
          e.sota_ref.eq_ignore_ascii_case(&reference) || e.pota_ref.eq_ignore_ascii_case(&reference)
        })
        .count()
    })
  };

  let coord = |lat: f64, lon: f64| format!("{:.4}, {:.4}", lat, lon);

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.sota-pota-reference-lookup")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("radio.enter-a-sota-summit")}
      </p>
      <div class="flex flex-wrap items-center gap-3 p-4">
        <div class="flex rounded-lg border p-0.5">
          <button
            type="button"
            class=move || {
              if !pota.get() {
                "rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
              } else {
                "rounded-md px-3 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              }
            }
            on:click=move |_| pota.set(false)
          >
            {move || t("radio.sota-summits")}
          </button>
          <button
            type="button"
            class=move || {
              if pota.get() {
                "rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
              } else {
                "rounded-md px-3 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              }
            }
            on:click=move |_| pota.set(true)
          >
            {move || t("radio.pota-parks")}
          </button>
        </div>
        <Input
          value=code
          on_change=Callback::new(move |v: String| code.set(v))
          placeholder=Signal::derive(move || {
            if pota.get() { t("radio.e-g-k-0001") } else { t("radio.e-g-g-sp") }
          })
          aria_label=Signal::derive(move || t("radio.reference-2"))
          class="max-w-xs font-mono uppercase"
        />
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| run())
        >
          {move || if loading.get() { t("log.looking-up") } else { t("log.look-up") }}
        </Button>
      </div>

      {move || {
        if failed.get() {
          return view! {
            <p class="px-4 pb-4 text-sm text-muted-foreground">
              {move || t("radio.lookup-failed-the-reference")}
            </p>
          }
          .into_any();
        }
        result.get().map(|r| {
          match r {
            LookupResult::Summit(s) => view! {
              <div class="space-y-1 border-t px-4 py-4 text-sm">
                <div class="flex flex-wrap items-baseline gap-x-2">
                  <span class="font-mono font-semibold">{s.reference.clone()}</span>
                  <span class="font-medium">{s.name}</span>
                  <span class="text-xs text-muted-foreground">{s.region}</span>
                </div>
                <div class="flex flex-wrap gap-x-5 text-xs text-muted-foreground">
                  <span>{t("radio.altitude")} <b class="tabular-nums text-foreground">{s.altitude_m} " m"</b></span>
                  <span>{t("radio.points")} <b class="tabular-nums text-foreground">{s.points}</b></span>
                  <span class="tabular-nums">{coord(s.latitude, s.longitude)}</span>
                </div>
                <div class="pt-1 text-xs text-muted-foreground">
                  {t("radio.logged-locally")} <b class="tabular-nums text-foreground">{local_count(s.reference)}</b> {t("radio.times")}
                </div>
              </div>
            }.into_any(),
            LookupResult::Park(p) => view! {
              <div class="space-y-1 border-t px-4 py-4 text-sm">
                <div class="flex flex-wrap items-baseline gap-x-2">
                  <span class="font-mono font-semibold">{p.reference.clone()}</span>
                  <span class="font-medium">{p.name}</span>
                  <span class="text-xs text-muted-foreground">{p.entity}</span>
                </div>
                <div class="flex flex-wrap gap-x-5 text-xs text-muted-foreground">
                  <span>{t("radio.grid")} <b class="font-mono text-foreground">{p.grid}</b></span>
                  <span class="tabular-nums">{coord(p.latitude, p.longitude)}</span>
                </div>
                <div class="pt-1 text-xs text-muted-foreground">
                  {t("radio.logged-locally")} <b class="tabular-nums text-foreground">{local_count(p.reference)}</b> {t("radio.times")}
                </div>
              </div>
            }.into_any(),
          }
        })
        .into_any()
      }}

      <p class="px-4 pb-4 text-xs text-muted-foreground">
        {move || t("radio.data-from-the-public-2")}
      </p>
    </section>
  }
}
