//! 通联日志统计概览：总数 / 不同呼号 / DXCC / QSL 确认，模式与波段分布、DXCC 实体、奖项与网格地图。

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;

use crate::ui::Stat;

use super::awards_panel::AwardsPanel;
use super::bar_list::BarList;
use super::grid_map::GridMap;
use super::{Logbook, StationInfo};
use crate::i18n::{t, tf};

#[component]
pub(super) fn LogStatsPanel(
  logbook: RwSignal<Logbook>,
  station: RwSignal<StationInfo>,
) -> impl IntoView {
  view! {
    {move || {
      let entries = logbook.get().entries;
      if entries.is_empty() {
        return view! { <div></div> }.into_any();
      }
      let mut by_mode: HashMap<String, usize> = HashMap::new();
      let mut by_band: HashMap<String, usize> = HashMap::new();
      let mut dxcc: Vec<String> = Vec::new();
      let mut grids: Vec<String> = Vec::new();
      let mut calls: HashSet<String> = HashSet::new();
      let confirmed = entries.iter().filter(|e| e.qsl_rcvd).count();
      for e in &entries {
        *by_mode.entry(e.mode.clone()).or_default() += 1;
        let b = e.band_label();
        if !b.is_empty() {
          *by_band.entry(b).or_default() += 1;
        }
        if let Some(entity) = e.entity()
          && !dxcc.iter().any(|d| d == entity.name)
        {
          dxcc.push(entity.name.to_owned());
        }
        if !e.gridsquare.is_empty() && !grids.contains(&e.gridsquare) {
          grids.push(e.gridsquare.clone());
        }
        calls.insert(e.callsign.to_uppercase());
      }
      let mut modes: Vec<(String, usize)> = by_mode.into_iter().collect();
      let mut bands: Vec<(String, usize)> = by_band.into_iter().collect();
      modes.sort_by_key(|a| std::cmp::Reverse(a.1));
      bands.sort_by_key(|a| std::cmp::Reverse(a.1));
      dxcc.sort();
      grids.sort();
      let (total, n_calls, n_dxcc) = (entries.len(), calls.len(), dxcc.len());
      let pending = ham_web_core::logbook::pending_qsl(&entries);
      view! {
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("通联总数") value=move || total />
          <Stat label=t("不同呼号") value=move || n_calls />
          <Stat label=t("DXCC 实体") value=move || n_dxcc />
          <Stat label=t("QSL 已确认") value=move || confirmed />
        </div>
        <section class="grid gap-4 sm:grid-cols-2">
          <div class="rounded-xl border bg-card p-4">
            <h3 class="mb-3 text-sm font-semibold">{move || t("按模式")}</h3>
            <BarList items=modes />
          </div>
          <div class="rounded-xl border bg-card p-4">
            <h3 class="mb-3 text-sm font-semibold">{move || t("按波段")}</h3>
            <BarList items=bands />
          </div>
        </section>
        <section class="rounded-xl border bg-card p-4">
          <h3 class="mb-2 text-sm font-semibold">
            {move || t("DXCC 实体")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tf("已通联 {} 个", &[&n_dxcc.to_string()])}
            </span>
          </h3>
          {if dxcc.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("暂无已识别实体，添加通联记录后自动统计。")}</div> }.into_any()
          } else {
            view! {
              <div class="flex flex-wrap gap-1.5">
                {dxcc
                  .into_iter()
                  .map(|e| {
                    view! {
                      <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs">{e}</span>
                    }
                  })
                  .collect_view()}
              </div>
            }
            .into_any()
          }}
        </section>
        <AwardsPanel entries=entries.clone() />
        <section class="rounded-xl border bg-card p-4">
          <h3 class="mb-2 text-sm font-semibold">
            {move || t("待确认 QSL")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tf("{} 条待追卡", &[&pending.len().to_string()])}
            </span>
          </h3>
          {if pending.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("已寄出的 QSL 都确认了，很棒！")}</div> }.into_any()
          } else {
            view! {
              <div class="divide-y">
                {pending
                  .iter()
                  .map(|e| {
                    let via = if e.qsl_sent {
                      "纸卡"
                    } else if e.lotw_sent {
                      "LoTW"
                    } else {
                      "eQSL"
                    };
                    view! {
                      <div class="flex items-center gap-2 py-1.5 text-sm">
                        <span class="font-mono font-medium">{e.callsign.clone()}</span>
                        <span class="text-xs text-muted-foreground">
                          {e.date.clone()} " · " {e.band_label()} " · " {e.mode.clone()}
                        </span>
                        <span class="ml-auto rounded-full border px-2 py-0.5 text-[10px] text-muted-foreground">{via}</span>
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            }
            .into_any()
          }}
        </section>
        <section class="rounded-xl border bg-card p-4">
          <h3 class="mb-2 text-sm font-semibold">
            {move || t("已通联网格")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tf("{} 个", &[&grids.len().to_string()])}
            </span>
          </h3>
          {if grids.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("暂无网格记录，添加带网格的通联后自动统计。")}</div> }.into_any()
          } else {
            view! {
              <div class="space-y-3">
                <GridMap entries=entries.clone() station_grid=station.get_untracked().gridsquare.clone() />
                <div class="flex flex-wrap gap-1.5">
                  {grids
                    .into_iter()
                    .map(|g| {
                      view! {
                        <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 font-mono text-xs">{g}</span>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            }
            .into_any()
          }}
        </section>
      }
      .into_any()
    }}
  }
}
