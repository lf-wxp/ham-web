use ham_web_core::solar::{PropagationLevel, propagation_level};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::SectionCard;
use crate::data;
use crate::ui::{Button, Size, Variant};

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
    // 整卡可点（`href`）：标题链接铺满整张卡。不能把卡片本身做成一个 `<a>` —— 里面有「重试」
    // 按钮，链接嵌按钮是无效的交互嵌套；按钮自己用 `relative z-10` 浮在铺满的链接之上。
    // 右上角的「查看详情 →」只是视觉提示，不是第二个链接（两个指向同一页的链接对读屏是噪声）。
    <SectionCard
      href="/solar"
      title=Signal::derive(move || t("home.live-propagation"))
      meta=ViewFn::from(move || view! { <span aria-hidden="true">{t("home.details")}</span> })
    >
      <div class="space-y-3 p-4">
        <div class="grid grid-cols-3 gap-3">
          <PvMetric label=Signal::derive(move || t("home.k-index")) value=k loading=loading />
          <PvMetric label=Signal::derive(move || t("home.solar-flux")) value=sfi loading=loading />
          <PvMetric label=Signal::derive(move || t("home.sunspots")) value=ssn loading=loading />
        </div>
        <div class="border bg-inset px-3 py-2 text-sm">
          {move || {
            match k.get() {
              Some(kv) => {
                let lvl = propagation_level(kv, ssn.get().unwrap_or(0.0), sfi.get().unwrap_or(0.0));
                let color = match lvl {
                  PropagationLevel::Excellent => "text-win-text",
                  PropagationLevel::Good => "text-xp-text",
                  PropagationLevel::Fair => "text-gold-text",
                  PropagationLevel::Poor => "text-hp-text",
                };
                view! {
                  <span>{move || t("home.conditions")} <span class=color>{move || crate::data::core_text_tracked(lvl.label())}</span></span>
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
    </SectionCard>
  }
}
