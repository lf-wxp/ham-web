use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;

/// Club Log 实时榜返回结构。
#[derive(Deserialize, Clone)]
struct WantedEntry {
  rank: u32,
  adif: u32,
}

#[derive(Deserialize)]
struct WantedPayload {
  entries: Vec<WantedEntry>,
}

/// Club Log 实时最稀有榜卡片。
#[component]
pub(super) fn LiveRanking() -> impl IntoView {
  let entries = RwSignal::new(Vec::<WantedEntry>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);

  spawn_local(async move {
    match data::fetch_external_json::<WantedPayload>("/api/most-wanted").await {
      Ok(p) => entries.set(p.entries),
      Err(_) => failed.set(true),
    }
    loading.set(false);
  });

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">"Club Log 实时最稀有榜（Top 15）"</h2>
      <div class="p-4">
        {move || {
          if loading.get() {
            return view! { <p class="text-sm text-muted-foreground">"正在获取实时榜单…"</p> }.into_any();
          }
          if failed.get() || entries.get().is_empty() {
            return view! {
              <p class="text-sm text-muted-foreground">
                "实时榜单暂不可用，以下为内置参考清单（按稀有度排序）。"
              </p>
            }
            .into_any();
          }
          let list = entries.get();
          view! {
            <div class="flex flex-wrap gap-2">
              {list
                .iter()
                .map(|e| {
                  view! {
                    <span class="rounded-lg border bg-muted/40 px-2.5 py-1 text-xs tabular-nums">
                      <span class="font-semibold text-primary">{format!("#{}", e.rank)}</span>
                      " DXCC " <span class="font-mono">{e.adif}</span>
                    </span>
                  }
                })
                .collect_view()}
            </div>
          }
          .into_any()
        }}
        <p class="mt-3 text-xs text-muted-foreground">
          "数据来自 Club Log，编号为 ADIF DXCC 实体编号（可对照 DXCC 实体表）。"
        </p>
      </div>
    </section>
  }
}
