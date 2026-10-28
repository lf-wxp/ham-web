use ham_web_core::solar::{
  CONDITIONS, CYCLE_NOTES, PropagationLevel, SOLAR_INDICES, propagation_level,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::data::kt_tracked;
use crate::ui::{Button, Size, Variant};
use crate::util::set_title;

use super::alerts_card::AlertsCard;
use super::conditions_table::ConditionsTable;
use super::flux_trend::FluxTrend;
use super::kp_forecast_card::KpForecastCard;
use super::metric_card::MetricCard;
use super::xray_trend::XrayTrend;
use super::{
  BandCond, KP_URL, KpEntry, SOLAR_URL, SolarApi, SolarEntry, XrayApi, XrayPoint, fmt_iso_time,
};
use crate::i18n::{t, tf};

#[component]
pub fn SolarPage() -> impl IntoView {
  set_title("shell.solar-data");
  // 本页渲染 `crates/core` 的中文原文（`kt_tracked`），译文词典要自己挂一次拉取，
  // 否则以英文 / 西语直接打开本页时词典永远不来、这些文案停在中文。
  data::use_knowledge_i18n();

  let k = RwSignal::new(None::<f64>);
  let sfi = RwSignal::new(None::<f64>);
  let ssn = RwSignal::new(None::<f64>);
  let a = RwSignal::new(None::<f64>);
  let xray = RwSignal::new(String::new());
  // 更新时间的原文；`kp_fallback` 为真表示它来自 NOAA 降级路径，展示时要套「Kp 更新于 … UTC」。
  // 不能在落地时就拼成译文存进来：切语言后那句话会停在旧语言。
  let updated = RwSignal::new(String::new());
  let kp_fallback = RwSignal::new(false);
  let conditions = RwSignal::new(Vec::<BandCond>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);
  let history = RwSignal::new(Vec::<(String, f64)>::new());
  let xray_series = RwSignal::new(Vec::<XrayPoint>::new());
  let xray_flux = RwSignal::new(None::<f64>);
  let xray_class = RwSignal::new(String::new());

  let refresh = RwSignal::new(0u32);
  let load = move || {
    refresh.update(|g| *g = g.wrapping_add(1));
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

      // GOES X 射线通量（耀斑实时曲线，走服务端代理）
      if let Ok(api) = data::fetch_external_json::<XrayApi>("/api/xray").await {
        xray_flux.set(api.flux);
        xray_class.set(api.flare_class);
        xray_series.set(api.series);
      }

      // 优先走服务端代理（HamQSL，含 A/K 指数与各波段条件）
      if let Ok(api) = data::fetch_external_json::<SolarApi>("/api/solar").await {
        k.set(api.k_index.map(f64::from));
        sfi.set(api.solar_flux.map(f64::from));
        ssn.set(api.sunspots.map(f64::from));
        a.set(api.a_index.map(f64::from));
        xray.set(api.xray.unwrap_or_default());
        updated.set(api.updated.unwrap_or_default());
        kp_fallback.set(false);
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
        updated.set(fmt_iso_time(&last.time_tag).to_string());
        kp_fallback.set(true);
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
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.solar-data")
        subtitle=move || t("radio.solar-indices-propagation-conditions")
        actions=ViewFn::from(move || {
          view! {
            <Button
              variant=Variant::Outline
              size=Size::Sm
              loading=loading
              on_click=Callback::new(move |_| load())
            >
              {move || if loading.get() { t("radio.refreshing") } else { t("exam.refresh") }}
            </Button>
          }
        })
      />

      <PageContainer>
        <section class="pxl-card border bg-card">
          <h2 class="flex items-center justify-between pxl-section-title">
            {move || t("radio.live-solar-activity")}
            <span class="text-xs font-normal normal-case text-muted-foreground">
              {move || {
                let mut parts = Vec::new();
                if !updated.get().is_empty() {
                  parts.push(if kp_fallback.get() {
                    tf("radio.kp-updated-utc", &[&updated.get()])
                  } else {
                    updated.get()
                  });
                }
                if !xray.get().is_empty() {
                  parts.push(tf("radio.x-ray-flux", &[&xray.get().to_string()]));
                }
                if parts.is_empty() {
                  t("radio.data-from-hamqsl-noaa")
                } else {
                  parts.join(" · ")
                }
              }}
            </span>
          </h2>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-4">
            <MetricCard label=Signal::derive(move || t("home.k-index")) value=k unit=Signal::derive(move || t("radio.0-9-lower-is")) loading=loading failed=failed />
            <MetricCard label=Signal::derive(move || t("tools.solar-flux-sfi")) value=sfi unit=Signal::derive(move || t("radio.10-7-cm-flux")) loading=loading failed=failed />
            <MetricCard label=Signal::derive(move || t("radio.sunspot-number-ssn")) value=ssn unit=Signal::derive(move || t("radio.relative-number")) loading=loading failed=failed />
            <MetricCard label=Signal::derive(move || t("radio.a-index")) value=a unit=Signal::derive(move || t("radio.daily-geomagnetic-index")) loading=loading failed=failed />
          </div>
          <div class="grid grid-cols-2 gap-3 px-4 pb-4 sm:grid-cols-4">
            <div class="col-span-2 border bg-inset p-3 text-center sm:col-span-4">
              <div class="text-xs text-muted-foreground">{move || t("radio.propagation-conditions-from-k")}</div>
              <div class="mt-1 pxl-stat">
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
                      view! { <span class=color>{move || data::kt_tracked(lvl.label())}</span> }.into_any()
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
                    {move || t("radio.live-data-is-unavailable")}
                  </p>
                }
              })
          }}
        </section>

        <KpForecastCard refresh=refresh />

        <AlertsCard />

        {move || {
          let conds = conditions.get();
          (!conds.is_empty()).then(|| {
            view! {
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">{move || t("radio.band-conditions")}</h2>
                <div class="p-4">
                  <ConditionsTable conditions=conds />
                </div>
              </section>
            }
          })
        }}

        {move || {
          let series = xray_series.get();
          (!series.is_empty()).then(|| {
            view! {
              <XrayTrend series=series.clone() flux=xray_flux.get() flare_class=xray_class.get() />
            }
          })
        }}

        {move || {
          let hist = history.get();
          (!hist.is_empty()).then(|| view! { <FluxTrend history=hist /> })
        }}

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.solar-indices")}</h2>
          <div class="divide-y">
            {SOLAR_INDICES
              .iter()
              .map(|&(name, range, desc)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[10rem_12rem_1fr]">
                    <div>{move || kt_tracked(name)}</div>
                    <div class="font-mono text-xs text-muted-foreground">{move || kt_tracked(range)}</div>
                    <div class="text-sm text-muted-foreground">{move || kt_tracked(desc)}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.propagation-condition-scale")}</h2>
          <div class="grid gap-3 p-4 sm:grid-cols-2">
            {CONDITIONS
              .iter()
              .map(|&(level, desc)| {
                view! {
                  <div class="border bg-inset p-3">
                    <div>{move || crate::data::core_text_tracked(level)}</div>
                    <div class="mt-1 text-sm text-muted-foreground">{move || kt_tracked(desc)}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.solar-cycle")}</h2>
          <ul class="space-y-2 p-4">
            {CYCLE_NOTES
              .iter()
              .map(|note| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(note)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </PageContainer>
    </div>
  }
}
