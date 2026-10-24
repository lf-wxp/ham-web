use std::collections::HashSet;

use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use ham_web_core::solar::{PropagationLevel, propagation_level};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::SectionCard;
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
    PropagationLevel::Excellent => ("优秀", "bg-win/15 text-win-text"),
    PropagationLevel::Good => ("良好", "bg-win/15 text-win-text"),
    PropagationLevel::Fair => ("一般", "bg-gold/15 text-gold-text"),
    PropagationLevel::Poor => ("差", "bg-destructive/15 text-hp-text"),
  }
}

/// 由太阳通量给出波段建议。
fn band_hint(sfi: Option<f64>, k: Option<f64>) -> Option<(&'static str, &'static str)> {
  if k.is_some_and(|k| k >= 5.0) {
    return Some(("磁暴期间，暂缓追台", "text-muted-foreground"));
  }
  match sfi {
    Some(s) if s >= 130.0 => Some(("高波段（10/12/15m）活跃", "text-win-text")),
    Some(s) if s >= 100.0 => Some(("中波段（17/20m）稳定", "text-win-text")),
    Some(_) => Some(("低波段（40/80m）更稳", "text-gold-text")),
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

  // 没有可展示的内容就什么都不渲染：以前返回一个空 `<div>`，它在首页的 `space-y-6` 里
  // 也占一份 24px 间距，卡片之间平白多出一道空隙。
  if missing.is_empty() && level().is_none() {
    return ().into_any();
  }

  // 统一用 `SectionCard`：以前这里整张手写成 `<a>`，标题是普通字、内边距 `p-5`、悬停上抬 4px
  // 还带主色阴影，和同页其它卡片是三套写法。整卡可点（`href`）的反馈由 `motion-lift` 统一给。
  view! {
    <SectionCard
      href="/most-wanted"
      title=Signal::derive(move || t("radio.real-time-chase-suggestions"))
      meta=ViewFn::from(move || {
        level()
          .map(|lv| {
            let (label, class) = level_badge(lv);
            view! {
              <span class=format!("pxl-badge {class}")>
                {tf("radio.propagation", &[&t(label)])}
              </span>
            }
          })
      })
    >
      <div class="space-y-3 p-4">
        {move || {
          band_hint(sfi.get(), k.get()).map(|(text, class)| {
            view! {
              <p class=format!("text-xs {}", class)>{t(text)}</p>
            }
          })
        }}

        {(!missing.is_empty()).then(|| {
          view! {
            <div class="space-y-2">
              <p class="text-xs text-muted-foreground">
                {move || t("radio.based-on-your-log")}
              </p>
              <div class="space-y-1.5">
                {top
                  .into_iter()
                  .map(|(p, name, rarity)| {
                    view! {
                      <div class="flex items-center justify-between text-sm">
                        <span class="font-mono text-xs text-muted-foreground">{p}</span>
                        <span class="min-w-0 flex-1 truncate px-2">{name}</span>
                        <span class="shrink-0 border bg-muted px-2 py-0.5 text-xs">{rarity}</span>
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            </div>
          }
        })}

        <div class="text-xs text-primary">
          {move || if missing.is_empty() { t("radio.view-propagation") } else { t("radio.track-them") }}
        </div>
      </div>
    </SectionCard>
  }
  .into_any()
}
