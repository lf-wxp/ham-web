use ham_web_core::solar::{PropagationLevel, propagation_level};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::ui::{
  Button, CARD_HEADER, Size, Variant, card_content_class, card_title_class, flat_card_class,
};

use super::pv_metric::PvMetric;
use crate::i18n::t;

/// 服务端 `/api/solar` 返回的传播条件摘要。
#[derive(Deserialize)]
struct SolarSummary {
  k_index: Option<i32>,
  solar_flux: Option<i32>,
  sunspots: Option<i32>,
}

/// 实时传播条件卡片（标题与「查看详情」链接到 /solar）。
///
/// 拉取失败要能和「暂时没有数据」分开说，并给一次就地重试 —— 原来是
/// `if let Ok(..)` 把错误吞成「数据暂不可用」，用户既看不出是网断了，也只能
/// 靠刷新整页重来。
#[component]
pub(super) fn PropagationWidget() -> impl IntoView {
  let k = RwSignal::new(None::<f64>);
  let sfi = RwSignal::new(None::<f64>);
  let ssn = RwSignal::new(None::<f64>);
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);

  let load = move || {
    loading.set(true);
    failed.set(false);
    spawn_local(async move {
      match data::fetch_external_json::<SolarSummary>("/api/solar").await {
        Ok(api) => {
          k.set(api.k_index.map(f64::from));
          sfi.set(api.solar_flux.map(f64::from));
          ssn.set(api.sunspots.map(f64::from));
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };
  load();

  view! {
    // 整卡可点：标题链接用 `after:inset-0` 铺满整张卡（stretched link）。不能像以前那样
    // 直接把卡片做成一个 `<a>` —— 里面现在有「重试」按钮，链接嵌按钮是无效的交互嵌套；
    // 而热区缩到只剩标题那一行，又是一次实打实的退步。
    <div data-slot="card" class=flat_card_class("group relative transition-colors hover:bg-accent/40")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <a href="/solar" class="after:absolute after:inset-0 group-hover:underline">
            {move || t("home.live-propagation")}
          </a>
          // 视觉提示而非第二个链接：两个指向同一页的链接对读屏是噪声，点击由上面的标题链接接管。
          <span class="text-sm font-normal text-muted-foreground group-hover:underline" aria-hidden="true">
            {move || t("home.details")}
          </span>
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        <div class="grid grid-cols-3 gap-3">
          <PvMetric label=t("home.k-index") value=k loading=loading />
          <PvMetric label=t("home.solar-flux") value=sfi loading=loading />
          <PvMetric label=t("home.sunspots") value=ssn loading=loading />
        </div>
        <div class="border bg-muted/40 px-3 py-2 text-sm">
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
                  <span>{move || t("home.conditions")} <span class=format!("font-semibold {color}")>{move || t(lvl.label())}</span></span>
                }
                .into_any()
              }
              None if loading.get() => view! { <span>{move || t("home.fetching-conditions-u-2026")}</span> }.into_any(),
              None if failed.get() => {
                view! {
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <span>{move || t("home.propagation-fetch-failed")}</span>
                    <Button
                      variant=Variant::Outline
                      size=Size::Sm
                      class="relative z-10"
                      on_click=Callback::new(move |_| load())
                    >
                      {move || t("exam.retry")}
                    </Button>
                  </div>
                }
                .into_any()
              }
              None => view! { <span>{move || t("home.propagation-data-unavailable-u")}</span> }.into_any(),
            }
          }}
        </div>
      </div>
    </div>
  }
}
