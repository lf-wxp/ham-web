use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;

use super::{Alert, level_color};
use crate::i18n::t;

/// 空间天气警报卡片。
#[component]
pub(super) fn AlertsCard() -> impl IntoView {
  let alerts = RwSignal::new(Vec::<Alert>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);

  spawn_local(async move {
    match data::fetch_external_json::<Vec<Alert>>("/api/alerts").await {
      Ok(a) => alerts.set(a),
      Err(_) => failed.set(true),
    }
    loading.set(false);
  });

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("空间天气警报")}</h2>
      <div class="p-2">
        {move || {
          if loading.get() {
            return view! {
              <p class="px-3 py-6 text-center text-sm text-muted-foreground">{move || t("正在获取警报…")}</p>
            }
            .into_any();
          }
          if failed.get() || alerts.get().is_empty() {
            return view! {
              <p class="px-3 py-6 text-center text-sm text-muted-foreground">{move || t("当前无有效警报或数据暂不可用。")}</p>
            }
            .into_any();
          }
          let list = alerts.get();
          view! {
            <div class="divide-y">
              {list
                .iter()
                .take(10)
                .map(|a| {
                  let color = level_color(a.level.as_str());
                  let msg: String = a.message.chars().take(140).collect();
                  let msg = if a.message.chars().count() > 140 { format!("{msg}…") } else { msg };
                  view! {
                    <div class="px-3 py-2.5">
                      <div class="flex flex-wrap items-center gap-2">
                        {if a.level.is_empty() {
                          view! { <span></span> }.into_any()
                        } else {
                          view! {
                            <span class=format!("rounded bg-muted px-1.5 py-0.5 font-mono text-xs font-semibold {color}")>
                              {a.level.clone()}
                            </span>
                          }
                          .into_any()
                        }}
                        <span class="font-mono text-xs text-muted-foreground">{a.product_id.clone()}</span>
                        <span class="ml-auto text-xs text-muted-foreground">{a.issue_time.clone()}</span>
                      </div>
                      <p class="mt-1 text-xs text-muted-foreground">{msg}</p>
                    </div>
                  }
                })
                .collect_view()}
            </div>
          }
          .into_any()
        }}
      </div>
    </section>
  }
}
