use ham_web_core::solar::{
  CONDITIONS, CYCLE_NOTES, PropagationLevel, SOLAR_INDICES, propagation_level,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::data;
use crate::icons::{Icon, IconKind};
use crate::ui::{Size, Variant, button_class};
use crate::util::set_title;

use super::alerts_card::AlertsCard;
use super::conditions_table::ConditionsTable;
use super::flux_trend::FluxTrend;
use super::metric_card::MetricCard;
use super::{BandCond, KP_URL, KpEntry, SOLAR_URL, SolarApi, SolarEntry, fmt_iso_time};

#[component]
pub fn SolarPage() -> impl IntoView {
  set_title("太阳活动");

  let k = RwSignal::new(None::<f64>);
  let sfi = RwSignal::new(None::<f64>);
  let ssn = RwSignal::new(None::<f64>);
  let a = RwSignal::new(None::<f64>);
  let xray = RwSignal::new(String::new());
  let updated = RwSignal::new(String::new());
  let conditions = RwSignal::new(Vec::<BandCond>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);
  let history = RwSignal::new(Vec::<(String, f64)>::new());

  let load = move || {
    loading.set(true);
    failed.set(false);
    spawn_local(async move {
      // 历史趋势（NOAA 月度太阳通量，近 12 个月）
      if let Ok(entries) = data::fetch_external_json::<Vec<SolarEntry>>(SOLAR_URL).await {
        let hist: Vec<(String, f64)> = entries
          .iter()
          .rev()
          .filter_map(|e| e.f107.filter(|v| *v > 0.0).map(|f| (e.time_tag.clone(), f)))
          .take(12)
          .collect();
        history.set(hist.into_iter().rev().collect());
      }

      // 优先走服务端代理（HamQSL，含 A/K 指数与各波段条件）
      if let Ok(api) = data::fetch_external_json::<SolarApi>("/api/solar").await {
        k.set(api.k_index.map(f64::from));
        sfi.set(api.solar_flux.map(f64::from));
        ssn.set(api.sunspots.map(f64::from));
        a.set(api.a_index.map(f64::from));
        xray.set(api.xray.unwrap_or_default());
        updated.set(api.updated.unwrap_or_default());
        conditions.set(api.conditions);
        loading.set(false);
        return;
      }

      // 降级：直接请求 NOAA
      let mut any = false;
      if let Ok(entries) = data::fetch_external_json::<Vec<KpEntry>>(KP_URL).await
        && let Some(last) = entries.last()
        && let Some(v) = last.estimated_kp.or_else(|| last.kp_index.map(f64::from))
      {
        k.set(Some(v));
        updated.set(format!("Kp 更新于 {} UTC", fmt_iso_time(&last.time_tag)));
        any = true;
      }
      if let Ok(entries) = data::fetch_external_json::<Vec<SolarEntry>>(SOLAR_URL).await {
        for e in entries.iter().rev() {
          if let (Some(f), Some(s)) = (e.f107.filter(|v| *v > 0.0), e.ssn.filter(|v| *v > 0.0)) {
            sfi.set(Some(f));
            ssn.set(Some(s));
            any = true;
            break;
          }
        }
      }
      if !any {
        failed.set(true);
      }
      loading.set(false);
    });
  };

  load();

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"太阳活动"</h1>
            <div class="text-xs text-muted-foreground">"太阳活动指数 · 传播条件 · 实时数据"</div>
          </div>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| load()
          >
            {move || {
              if loading.get() {
                view! {
                  <span class="inline-flex items-center gap-1.5">
                    <Icon kind=IconKind::Loader2 class="h-3.5 w-3.5 animate-spin" />
                    "刷新中"
                  </span>
                }
                .into_any()
              } else {
                "刷新".into_any()
              }
            }}
          </button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="flex items-center justify-between border-b px-4 py-3 text-sm font-semibold">
            "实时太阳活动"
            <span class="text-xs font-normal text-muted-foreground">
              {move || {
                let mut parts = Vec::new();
                if !updated.get().is_empty() {
                  parts.push(updated.get());
                }
                if !xray.get().is_empty() {
                  parts.push(format!("X 射线耀斑 {}", xray.get()));
                }
                if parts.is_empty() {
                  "数据来自 HamQSL / NOAA SWPC".to_owned()
                } else {
                  parts.join(" · ")
                }
              }}
            </span>
          </h2>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
            <MetricCard label="K 指数" value=k unit="0–9（越低越安静）" loading=loading failed=failed />
            <MetricCard label="太阳通量 SFI" value=sfi unit="10.7cm 流量" loading=loading failed=failed />
            <MetricCard label="太阳黑子数 SSN" value=ssn unit="相对数" loading=loading failed=failed />
            <MetricCard label="A 指数" value=a unit="日地磁指数" loading=loading failed=failed />
          </div>
          <div class="grid grid-cols-2 gap-3 px-4 pb-4 sm:grid-cols-4">
            <div class="col-span-2 rounded-lg border bg-muted/40 p-3 text-center sm:col-span-4">
              <div class="text-xs text-muted-foreground">"传播条件（按 K / SSN / SFI 综合判断）"</div>
              <div class="mt-1 text-2xl font-semibold tabular-nums">
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
                      view! { <span class=color>{lvl.label()}</span> }.into_any()
                    }
                    None if loading.get() => view! { <span>"…"</span> }.into_any(),
                    None => view! { <span>"—"</span> }.into_any(),
                  }
                }}
              </div>
            </div>
          </div>
          {move || {
            failed
              .get()
              .then(|| {
                view! {
                  <p class="px-4 pb-4 text-xs text-muted-foreground">
                    "实时数据暂不可用（可能因网络受限），以下为科普内容。"
                  </p>
                }
              })
          }}
        </section>

        <AlertsCard />

        {move || {
          let conds = conditions.get();
          (!conds.is_empty()).then(|| {
            view! {
              <section class="rounded-xl border bg-card">
                <h2 class="border-b px-4 py-3 text-sm font-semibold">"各波段传播条件"</h2>
                <div class="p-4">
                  <ConditionsTable conditions=conds />
                </div>
              </section>
            }
          })
        }}

        {move || {
          let hist = history.get();
          (!hist.is_empty()).then(|| view! { <FluxTrend history=hist /> })
        }}

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"太阳活动指数"</h2>
          <div class="divide-y">
            {SOLAR_INDICES
              .iter()
              .map(|&(name, range, desc)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[10rem_12rem_1fr]">
                    <div class="font-medium">{name}</div>
                    <div class="font-mono text-xs text-muted-foreground">{range}</div>
                    <div class="text-sm text-muted-foreground">{desc}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"传播条件分级"</h2>
          <div class="grid gap-3 p-4 sm:grid-cols-2">
            {CONDITIONS
              .iter()
              .map(|&(level, desc)| {
                view! {
                  <div class="rounded-lg border bg-muted/40 p-3">
                    <div class="font-medium">{level}</div>
                    <div class="mt-1 text-sm text-muted-foreground">{desc}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"太阳活动周期"</h2>
          <ul class="space-y-2 p-4">
            {CYCLE_NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*note}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </div>
    </div>
  }
}
