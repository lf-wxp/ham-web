use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;

/// DXCC 稀有度（/api/most-wanted）。
#[derive(Deserialize, Clone)]
struct WantedEntry {
  rank: u32,
  adif: u32,
}

/// `/api/most-wanted` 返回结构。
#[derive(Deserialize)]
struct WantedPayload {
  entries: Vec<WantedEntry>,
}

/// DXCC 稀有度摘要卡片（Top 5）。
#[component]
pub(super) fn WantedCard() -> impl IntoView {
  let entries = RwSignal::new(Vec::<WantedEntry>::new());
  let loading = RwSignal::new(true);

  let refresh = move || {
    spawn_local(async move {
      if let Ok(api) = data::fetch_external_json::<WantedPayload>("/api/most-wanted").await {
        entries.set(api.entries.into_iter().take(5).collect());
      }
      loading.set(false);
    });
  };

  refresh();
  set_interval(refresh, std::time::Duration::from_secs(60));

  view! {
    <a href="/most-wanted" class="rounded-xl border bg-card p-4 transition-colors hover:bg-accent/40">
      <div class="text-sm font-semibold">{move || t("DXCC 最稀有 Top 5")}</div>
      <div class="mt-2 space-y-1">
        {move || {
          if loading.get() {
            return view! { <div class="text-sm text-muted-foreground">{move || t("加载中…")}</div> }.into_any();
          }
          let list = entries.get();
          if list.is_empty() {
            return view! { <div class="text-sm text-muted-foreground">{move || t("暂不可用")}</div> }.into_any();
          }
          view! {
            {list
              .iter()
              .map(|e| {
                view! {
                  <div class="flex items-center gap-2 text-sm">
                    <span class="font-mono text-xs tabular-nums text-muted-foreground">{format!("#{}", e.rank)}</span>
                    <span class="font-mono text-sm">{format!("DXCC {}", e.adif)}</span>
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
