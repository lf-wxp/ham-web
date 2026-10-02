//! DXCC 世界地图：按通联日志把世界地图着色为「未通联 / 已通联 / 已确认」choropleth。

use std::collections::BTreeSet;

use ham_web_core::award_progress::AwardProgress;
use ham_web_core::dxcc_map::{DxccShapes, decode_binary};
use ham_web_core::log_stats::BAND_ORDER;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::components::common::{Legend, PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::pages::map::{DxccOverlay, MapView};
use crate::util::set_title;

#[component]
pub fn DxccMapPage() -> impl IntoView {
  set_title(&t("DXCC 世界地图"));

  let store = use_log_store();
  let shapes = RwSignal::new(DxccShapes::default());
  let loading = RwSignal::new(true);
  let load_error = RwSignal::new(false);

  spawn_local(async move {
    match data::fetch_bytes("/dxcc-entities.bin").await {
      Ok(bytes) => match decode_binary(&bytes) {
        Some(s) => shapes.set(s),
        None => load_error.set(true),
      },
      Err(_) => load_error.set(true),
    }
    loading.set(false);
  });

  let progress = Memo::new(move |_| AwardProgress::from_entries(&store.logbook.get().entries));
  // 选中波段（None = 全部）；按波段切换着色与统计。
  let selected_band = RwSignal::new(None::<String>);
  let worked = Signal::derive(move || {
    let p = progress.get();
    match selected_band.get() {
      Some(b) => p
        .dxcc_by_band
        .get(&b)
        .map_or_else(BTreeSet::new, |x| x.worked.clone()),
      None => p.dxcc.worked.clone(),
    }
  });
  let confirmed = Signal::derive(move || {
    let p = progress.get();
    match selected_band.get() {
      Some(b) => p
        .dxcc_by_band
        .get(&b)
        .map_or_else(BTreeSet::new, |x| x.confirmed.clone()),
      None => p.dxcc.confirmed.clone(),
    }
  });
  // 日志里出现过的波段（按低频到高频排序），用于波段切换 chips。
  let bands = Memo::new(move |_| {
    let p = progress.get();
    let mut out: Vec<String> = BAND_ORDER
      .iter()
      .filter(|b| p.dxcc_by_band.contains_key(**b))
      .map(|b| (*b).to_owned())
      .collect();
    for b in p.dxcc_by_band.keys() {
      if !BAND_ORDER.contains(&b.as_str()) {
        out.push(b.clone());
      }
    }
    out
  });
  let total_entities = ham_web_core::dxcc::entities().len();

  // 点击实体 → 跳转日志页并按实体名过滤。
  let navigate = use_navigate();
  let on_entity_click = Callback::new(move |dxcc: u16| {
    if let Some(name) = ham_web_core::dxcc::entity_by_dxcc(dxcc).map(|e| e.name) {
      navigate(&format!("/log?q={name}"), NavigateOptions::default());
    }
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("DXCC 世界地图"))
        subtitle=Signal::derive(move || t("按通联日志着色 · 已通联 / 已确认 / 未通联"))
        actions=ViewFn::from(move || {
          view! {
            <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
              {move || tf("已通联 {} / {} 实体", &[&(progress.get().dxcc.worked.len()).to_string(), &total_entities.to_string()])}
            </span>
          }
        })
      />
      <PageContainer>
        // 统计条（跟随波段选择）
        <section class="grid grid-cols-3 gap-2">
          <div class="rounded-xl border bg-card p-3 text-center">
            <div class="text-xl font-semibold tabular-nums">{move || worked.get().len()}</div>
            <div class="mt-0.5 text-xs text-muted-foreground">{move || t("已通联")}</div>
          </div>
          <div class="rounded-xl border bg-card p-3 text-center">
            <div class="text-xl font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
              {move || confirmed.get().len()}
            </div>
            <div class="mt-0.5 text-xs text-muted-foreground">{move || t("已确认")}</div>
          </div>
          <div class="rounded-xl border bg-card p-3 text-center">
            <div class="text-xl font-semibold tabular-nums">
              {move || total_entities.saturating_sub(worked.get().len())}
            </div>
            <div class="mt-0.5 text-xs text-muted-foreground">{move || t("未通联")}</div>
          </div>
        </section>

        // 波段切换
        <div class="flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            class=move || if selected_band.get().is_none() {
              "rounded-full border bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
            } else {
              "rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent"
            }
            on:click=move |_| selected_band.set(None)
          >
            {move || t("全部")}
          </button>
          {move || {
            bands.get().into_iter().map(|b| {
              let label = b.clone();
              let sel_b = b.clone();
              let click_b = b.clone();
              view! {
                <button
                  type="button"
                  class=move || if selected_band.get().as_deref() == Some(sel_b.as_str()) {
                    "rounded-full border bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
                  } else {
                    "rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent"
                  }
                  on:click=move |_| selected_band.set(Some(click_b.clone()))
                >
                  {label.clone()}
                </button>
              }
            })
            .collect_view()
          }}
        </div>

        // 地图
        <section class="rounded-xl border bg-card p-4">
          {move || {
            if loading.get() {
              view! {
                <div class="py-12 text-center text-sm text-muted-foreground">
                  {move || t("加载地图数据…")}
                </div>
              }
              .into_any()
            } else if load_error.get() {
              view! {
                <div class="py-12 text-center text-sm text-muted-foreground">
                  {move || t("DXCC 边界数据加载失败，请运行 `cargo make dxcc-map` 生成 public/dxcc-entities.bin。")}
                </div>
              }
              .into_any()
            } else {
              view! {
                <MapView>
                  <DxccOverlay shapes=shapes worked=worked confirmed=confirmed on_entity_click=on_entity_click />
                </MapView>
              }
              .into_any()
            }
          }}

          // 图例
          <div class="mt-3 space-y-1.5">
            <Legend
              items=vec![
                ("h-3 w-3 rounded-sm bg-emerald-600/70", "已确认"),
                ("h-3 w-3 rounded-sm bg-primary/40", "已通联"),
                ("h-3 w-3 rounded-sm border bg-foreground/5", "未通联"),
              ]
            />
            <div class="text-xs text-muted-foreground">{move || t("滚轮/双指缩放 · 拖拽平移 · 双击复位 · 悬停查看实体")}</div>
          </div>
          <p class="mt-2 text-xs text-muted-foreground">
            {move || t("着色基于本地通联日志自动统计，仅保存在浏览器中。已确认指 QSL 已收到（纸卡 / LoTW / eQSL）。岛屿等无国界数据的实体以中心点标记。")}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}
