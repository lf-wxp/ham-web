use ham_web_core::solar::{PropagationLevel, propagation_level};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::ui::{CARD_HEADER, card_class, card_content_class, card_title_class};

use super::pv_metric::PvMetric;
use crate::i18n::t;

/// 服务端 `/api/solar` 返回的传播条件摘要。
#[derive(Deserialize)]
struct SolarSummary {
  k_index: Option<i32>,
  solar_flux: Option<i32>,
  sunspots: Option<i32>,
}

/// 实时传播条件卡片（链接到 /solar）。
#[component]
pub(super) fn PropagationWidget() -> impl IntoView {
  let k = RwSignal::new(None::<f64>);
  let sfi = RwSignal::new(None::<f64>);
  let ssn = RwSignal::new(None::<f64>);
  let loading = RwSignal::new(true);

  spawn_local(async move {
    if let Ok(api) = data::fetch_external_json::<SolarSummary>("/api/solar").await {
      k.set(api.k_index.map(f64::from));
      sfi.set(api.solar_flux.map(f64::from));
      ssn.set(api.sunspots.map(f64::from));
    }
    loading.set(false);
  });

  view! {
    <a href="/solar" data-slot="card" class=card_class("group transition-colors hover:bg-accent/40")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>{move || t("实时传播条件")}</span>
          <span class="text-sm font-normal text-muted-foreground">{move || t("查看详情 →")}</span>
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        <div class="grid grid-cols-3 gap-3">
          <PvMetric label=t("K 指数") value=k loading=loading />
          <PvMetric label=t("太阳通量") value=sfi loading=loading />
          <PvMetric label=t("黑子数") value=ssn loading=loading />
        </div>
        <div class="rounded-lg border bg-muted/40 px-3 py-2 text-sm">
          {move || {
            match k.get() {
              Some(kv) => {
                let lvl = propagation_level(kv, ssn.get().unwrap_or(0.0), sfi.get().unwrap_or(0.0));
                let color = match lvl {
                  PropagationLevel::Excellent => "text-emerald-700 dark:text-emerald-400",
                  PropagationLevel::Good => "text-sky-700 dark:text-sky-400",
                  PropagationLevel::Fair => "text-amber-700 dark:text-amber-400",
                  PropagationLevel::Poor => "text-red-700 dark:text-red-400",
                };
                view! {
                  <span>{move || t("传播条件：")} <span class=format!("font-semibold {color}")>{move || t(lvl.label())}</span></span>
                }
                .into_any()
              }
              None if loading.get() => view! { <span>{move || t("正在获取传播条件…")}</span> }.into_any(),
              None => view! { <span>{move || t("传播数据暂不可用，点击查看科普内容")}</span> }.into_any(),
            }
          }}
        </div>
      </div>
    </a>
  }
}
