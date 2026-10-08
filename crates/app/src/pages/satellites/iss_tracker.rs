use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;

/// `/api/iss` 返回结构。
#[derive(Deserialize, Clone)]
struct IssData {
  latitude: f64,
  longitude: f64,
  altitude: f64,
  velocity: f64,
  visibility: String,
}

/// 可见性文字映射。
fn visibility_label(v: &str) -> &'static str {
  match v {
    "daylight" => "日照中",
    "eclipsed" => "地影中",
    _ => "未知",
  }
}

/// ISS 国际空间站实时位置卡片。
#[component]
pub(super) fn IssTracker() -> impl IntoView {
  let iss = RwSignal::new(None::<IssData>);
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);

  spawn_local(async move {
    match data::fetch_external_json::<IssData>("/api/iss").await {
      Ok(d) => iss.set(Some(d)),
      Err(_) => failed.set(true),
    }
    loading.set(false);
  });

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.iss-live-position")}</h2>
      <div class="p-4">
        {move || {
          if loading.get() {
            return view! {
              <p class="px-3 py-6 text-center text-sm text-muted-foreground">{move || t("radio.fetching-iss-position")}</p>
            }
            .into_any();
          }
          if failed.get() {
            return view! {
              <p class="px-3 py-6 text-center text-sm text-muted-foreground">{move || t("radio.iss-data-is-unavailable")}</p>
            }
            .into_any();
          }
          match iss.get() {
            Some(d) => view! {
              <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <div class="rounded-lg border bg-muted/40 p-3 text-center">
                  <div class="text-xs text-muted-foreground">{move || t("radio.latitude")}</div>
                  <div class="mt-1 text-lg font-semibold tabular-nums">{format!("{:.2}°", d.latitude)}</div>
                </div>
                <div class="rounded-lg border bg-muted/40 p-3 text-center">
                  <div class="text-xs text-muted-foreground">{move || t("radio.longitude")}</div>
                  <div class="mt-1 text-lg font-semibold tabular-nums">{format!("{:.2}°", d.longitude)}</div>
                </div>
                <div class="rounded-lg border bg-muted/40 p-3 text-center">
                  <div class="text-xs text-muted-foreground">{move || t("radio.altitude-2")}</div>
                  <div class="mt-1 text-lg font-semibold tabular-nums">{format!("{:.0} km", d.altitude)}</div>
                </div>
                <div class="rounded-lg border bg-muted/40 p-3 text-center">
                  <div class="text-xs text-muted-foreground">{move || t("radio.speed")}</div>
                  <div class="mt-1 text-lg font-semibold tabular-nums">{format!("{:.1} km/s", d.velocity)}</div>
                </div>
              </div>
              <p class="mt-3 text-xs text-muted-foreground">
                {move || t("radio.current-state")} <span class="font-medium text-foreground">{move || t(visibility_label(d.visibility.as_str()))}</span>
                {move || t("radio.data-from-wheretheiss-at")}
              </p>
            }
            .into_any(),
            None => view! { <p class="px-3 py-6 text-center text-sm text-muted-foreground">{move || t("radio.no-iss-data-yet")}</p> }
              .into_any(),
          }
        }}
      </div>
    </section>
  }
}
