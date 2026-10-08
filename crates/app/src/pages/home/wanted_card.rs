use std::collections::HashSet;

use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use ham_web_core::solar::{PropagationLevel, propagation_level};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::util::storage;

/// `/api/solar` 返回的传播相关字段。
#[derive(Deserialize)]
struct SolarSummary {
  k_index: Option<f64>,
  solar_flux: Option<f64>,
  sunspots: Option<f64>,
}

/// 传播条件徽标（颜色 + 文案）。
fn level_badge(level: PropagationLevel) -> (&'static str, &'static str) {
  match level {
    PropagationLevel::Excellent => (
      "优秀",
      "bg-emerald-500/15 text-emerald-700 dark:text-emerald-400",
    ),
    PropagationLevel::Good => (
      "良好",
      "bg-emerald-500/15 text-emerald-700 dark:text-emerald-400",
    ),
    PropagationLevel::Fair => ("一般", "bg-amber-500/15 text-amber-800 dark:text-amber-400"),
    PropagationLevel::Poor => ("差", "bg-red-500/15 text-red-700 dark:text-red-400"),
  }
}

/// 由太阳通量给出波段建议。
fn band_hint(sfi: Option<f64>, k: Option<f64>) -> Option<(&'static str, &'static str)> {
  if k.is_some_and(|k| k >= 5.0) {
    return Some(("磁暴期间，暂缓追台", "text-muted-foreground"));
  }
  match sfi {
    Some(s) if s >= 130.0 => Some((
      "高波段（10/12/15m）活跃",
      "text-emerald-700 dark:text-emerald-400",
    )),
    Some(s) if s >= 100.0 => Some((
      "中波段（17/20m）稳定",
      "text-emerald-700 dark:text-emerald-400",
    )),
    Some(_) => Some(("低波段（40/80m）更稳", "text-amber-700 dark:text-amber-400")),
    None => None,
  }
}

/// 首页追台建议卡：结合实时太阳活动与本地日志缺口，给出「现在适合追什么」。
#[component]
pub fn WantedCard() -> impl IntoView {
  let logged: HashSet<String> = use_log_store()
    .logbook
    .get_untracked()
    .entries
    .iter()
    .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
    .collect();
  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();

  let missing: Vec<(&'static str, &'static str, &'static str)> = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| !done.iter().any(|x| x == *p) && !logged.contains(*p))
    .copied()
    .collect();

  // 实时太阳活动。
  let k = RwSignal::new(None::<f64>);
  let sfi = RwSignal::new(None::<f64>);
  let ssn = RwSignal::new(None::<f64>);
  spawn_local(async move {
    if let Ok(api) = data::fetch_external_json::<SolarSummary>("/api/solar").await {
      k.set(api.k_index);
      sfi.set(api.solar_flux);
      ssn.set(api.sunspots);
    }
  });

  let level = move || match (k.get(), sfi.get(), ssn.get()) {
    (Some(k), Some(sfi), Some(ssn)) => Some(propagation_level(k, ssn, sfi)),
    _ => None,
  };

  let top: Vec<(&'static str, &'static str, &'static str)> =
    missing.iter().take(3).copied().collect();

  if missing.is_empty() && level().is_none() {
    return view! { <div></div> }.into_any();
  }

  view! {
    <a
      href="/most-wanted"
      class="block rounded-2xl border bg-card p-5 shadow-sm transition-all duration-300 hover:-translate-y-1 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/10"
    >
      <div class="flex items-center justify-between gap-2">
        <div class="font-semibold tracking-tight">{move || t("radio.real-time-chase-suggestions")}</div>
        {move || {
          level().map(|lv| {
            let (label, class) = level_badge(lv);
            view! {
              <span class=format!("rounded-full px-2.5 py-0.5 text-xs font-medium {}", class)>
                {tf("radio.propagation", &[&t(label)])}
              </span>
            }
          })
        }}
      </div>

      {move || {
        band_hint(sfi.get(), k.get()).map(|(text, class)| {
          view! {
            <p class=format!("mt-1 text-xs {}", class)>{t(text)}</p>
          }
        })
      }}

      {(!missing.is_empty()).then(|| {
        view! {
          <div>
            <p class="mt-2 text-xs text-muted-foreground">
              {move || t("radio.based-on-your-log")}
            </p>
            <div class="mt-2 space-y-1.5">
              {top
                .into_iter()
                .map(|(p, name, rarity)| {
                  view! {
                    <div class="flex items-center justify-between text-sm">
                      <span class="font-mono text-xs text-muted-foreground">{p}</span>
                      <span class="min-w-0 flex-1 truncate px-2">{name}</span>
                      <span class="shrink-0 rounded-full bg-muted/50 px-2 py-0.5 text-[10px]">{rarity}</span>
                    </div>
                  }
                })
                .collect_view()}
            </div>
          </div>
        }
      })}

      <div class="mt-3 text-xs font-medium text-primary">
        {move || if missing.is_empty() { t("radio.view-propagation") } else { t("radio.track-them") }}
      </div>
    </a>
  }
  .into_any()
}
