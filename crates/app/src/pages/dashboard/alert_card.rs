use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::t;

/// 空间天气警报（/api/alerts）。
#[derive(Deserialize, Clone)]
struct Alert {
  level: String,
  product_id: String,
}

/// 空间天气警报摘要卡片。
#[component]
pub(super) fn AlertCard() -> impl IntoView {
  let alert = RwSignal::new(None::<Alert>);
  let loading = RwSignal::new(true);

  let refresh = move || {
    spawn_local(async move {
      if let Ok(list) = data::fetch_external_json::<Vec<Alert>>("/api/alerts").await {
        alert.set(list.into_iter().next());
      }
      loading.set(false);
    });
  };

  refresh();
  if let Ok(handle) = set_interval_with_handle(refresh, std::time::Duration::from_secs(60)) {
    on_cleanup(move || handle.clear());
  }

  view! {
    <a href="/solar" class="rounded-xl border bg-card p-4 transition-colors hover:bg-accent/40">
      <div class="text-sm font-semibold">{move || t("空间天气警报")}</div>
      <div class="mt-2">
        {move || match alert.get() {
          Some(a) if !a.level.is_empty() => view! {
            <span class="rounded bg-muted px-2 py-1 font-mono text-sm font-semibold text-destructive">{a.level.clone()}</span>
          }
          .into_any(),
          Some(a) => view! {
            <span class="font-mono text-sm text-muted-foreground">{a.product_id.clone()}</span>
          }
          .into_any(),
          None if loading.get() => view! { <span class="text-sm text-muted-foreground">{move || t("加载中…")}</span> }.into_any(),
          None => view! { <span class="text-sm text-muted-foreground">{move || t("当前无有效警报")}</span> }.into_any(),
        }}
      </div>
    </a>
  }
}
