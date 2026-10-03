use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;

/// DX spot（/api/spots）。
#[derive(Deserialize, Clone)]
struct Spot {
  dx: String,
  freq_khz: u32,
  time: String,
}

/// 频率 kHz → MHz（去尾随 0）。
fn fmt_freq(khz: u32) -> String {
  let mhz = khz as f64 / 1000.0;
  let mut s = format!("{mhz:.3}");
  while s.ends_with('0') {
    s.pop();
  }
  if s.ends_with('.') {
    s.pop();
  }
  format!("{s} MHz")
}

/// DX 热点摘要卡片（前 5 条）。
#[component]
pub(super) fn SpotsCard() -> impl IntoView {
  let spots = RwSignal::new(Vec::<Spot>::new());
  let loading = RwSignal::new(true);

  let refresh = move || {
    spawn_local(async move {
      if let Ok(list) = data::fetch_external_json::<Vec<Spot>>("/api/spots").await {
        spots.set(list.into_iter().take(5).collect());
      }
      loading.set(false);
    });
  };

  refresh();
  if let Ok(handle) = set_interval_with_handle(refresh, std::time::Duration::from_secs(60)) {
    on_cleanup(move || handle.clear());
  }

  view! {
    <a href="/dx-spots" class="rounded-xl border bg-card p-4 transition-colors hover:bg-accent/40">
      <div class="text-sm font-semibold">{move || t("DX 实时热点")}</div>
      <div class="mt-2 space-y-1">
        {move || {
          if loading.get() {
            return view! { <div class="text-sm text-muted-foreground">{move || t("加载中…")}</div> }.into_any();
          }
          let list = spots.get();
          if list.is_empty() {
            return view! { <div class="text-sm text-muted-foreground">{move || t("暂不可用")}</div> }.into_any();
          }
          view! {
            {list
              .iter()
              .map(|s| {
                view! {
                  <div class="flex items-center gap-2 text-sm">
                    <span class="font-mono font-semibold text-primary">{s.dx.clone()}</span>
                    <span class="font-mono text-xs tabular-nums text-muted-foreground">{fmt_freq(s.freq_khz)}</span>
                    <span class="ml-auto text-xs text-muted-foreground">{s.time.clone()}</span>
                  </div>
                }
              })
              .collect_view()}
          }
          .into_any()
        }}
      </div>
    </a>
  }
}
