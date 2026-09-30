use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;

/// 太阳活动（/api/solar）。
#[derive(Deserialize, Clone)]
struct SolarApi {
  k_index: Option<i32>,
  solar_flux: Option<i32>,
  sunspots: Option<i32>,
}

/// 太阳活动摘要卡片。
#[component]
pub(super) fn SolarCard() -> impl IntoView {
  let k = RwSignal::new(None::<i32>);
  let sfi = RwSignal::new(None::<i32>);
  let ssn = RwSignal::new(None::<i32>);
  let loading = RwSignal::new(true);

  let refresh = move || {
    spawn_local(async move {
      if let Ok(api) = data::fetch_external_json::<SolarApi>("/api/solar").await {
        k.set(api.k_index);
        sfi.set(api.solar_flux);
        ssn.set(api.sunspots);
      }
      loading.set(false);
    });
  };

  refresh();
  set_interval(refresh, std::time::Duration::from_secs(60));

  view! {
    <a href="/solar" class="rounded-xl border bg-card p-4 transition-colors hover:bg-accent/40">
      <div class="text-sm font-semibold">"太阳活动"</div>
      <div class="mt-2 grid grid-cols-3 gap-2 text-center">
        <div class="rounded-lg bg-muted/40 p-2">
          <div class="text-xs text-muted-foreground">"K 指数"</div>
          <div class="mt-0.5 text-lg font-semibold tabular-nums">
            {move || k.get().map_or_else(|| if loading.get() { "…" } else { "—" }.to_owned(), |v| v.to_string())}
          </div>
        </div>
        <div class="rounded-lg bg-muted/40 p-2">
          <div class="text-xs text-muted-foreground">"通量"</div>
          <div class="mt-0.5 text-lg font-semibold tabular-nums">
            {move || sfi.get().map_or_else(|| "—".to_owned(), |v| v.to_string())}
          </div>
        </div>
        <div class="rounded-lg bg-muted/40 p-2">
          <div class="text-xs text-muted-foreground">"黑子"</div>
          <div class="mt-0.5 text-lg font-semibold tabular-nums">
            {move || ssn.get().map_or_else(|| "—".to_owned(), |v| v.to_string())}
          </div>
        </div>
      </div>
    </a>
  }
}
