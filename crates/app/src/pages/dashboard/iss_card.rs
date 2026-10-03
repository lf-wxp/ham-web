use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;

/// ISS 位置（/api/iss）。
#[derive(Deserialize, Clone)]
struct IssData {
  latitude: f64,
  longitude: f64,
}

/// ISS 位置摘要卡片。
#[component]
pub(super) fn IssCard() -> impl IntoView {
  let iss = RwSignal::new(None::<IssData>);
  let loading = RwSignal::new(true);

  let refresh = move || {
    spawn_local(async move {
      if let Ok(d) = data::fetch_external_json::<IssData>("/api/iss").await {
        iss.set(Some(d));
      }
      loading.set(false);
    });
  };

  refresh();
  if let Ok(handle) = set_interval_with_handle(refresh, std::time::Duration::from_secs(60)) {
    on_cleanup(move || handle.clear());
  }

  view! {
    <a href="/satellites" class="rounded-xl border bg-card p-4 transition-colors hover:bg-accent/40">
      <div class="text-sm font-semibold">{move || t("ISS 国际空间站")}</div>
      <div class="mt-2 text-sm tabular-nums text-muted-foreground">
        {move || match iss.get() {
          Some(d) => format!("{:.2}°, {:.2}°", d.latitude, d.longitude),
          None if loading.get() => t("加载中…"),
          None => t("暂不可用"),
        }}
      </div>
    </a>
  }
}
