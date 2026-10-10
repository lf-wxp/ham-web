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
      <div class="text-sm font-semibold">{move || t("learning.space-weather-alerts")}</div>
      <div class="mt-2">
        {move || match alert.get() {
          // 文字用调色板校正过的「文字档」（亮 700 / 暗 300），而不是 `--destructive`：
          // 后者是给实心按钮做底色的，落在 `--muted` 底上只有 4.29 / 4.43，过不了 AA。
          Some(a) if !a.level.is_empty() => view! {
            <span class="rounded bg-muted px-2 py-1 font-mono text-sm font-semibold text-red-700 dark:text-red-300">{a.level.clone()}</span>
          }
          .into_any(),
          Some(a) => view! {
            <span class="font-mono text-sm text-muted-foreground">{a.product_id.clone()}</span>
          }
          .into_any(),
          None if loading.get() => view! { <span class="text-sm text-muted-foreground">{move || t("learning.loading")}</span> }.into_any(),
          None => view! { <span class="text-sm text-muted-foreground">{move || t("learning.no-active-alerts")}</span> }.into_any(),
        }}
      </div>
    </a>
  }
}
