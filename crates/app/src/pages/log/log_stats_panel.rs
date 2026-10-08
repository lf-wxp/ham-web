//! 通联日志统计概览：总数 / 不同呼号 / DXCC / QSL 确认，模式与波段分布、DXCC 实体、奖项与网格地图。

use leptos::prelude::*;

use ham_web_core::log_stats::{LogOverview, StatsScope, operators, overview, scoped_entries};

use crate::ui::{NativeSelect, SelectOption, Stat};

use super::awards_panel::AwardsPanel;
use super::bar_list::BarList;
use super::grid_map::GridMap;
use super::log_helpers::station_title;
use super::{Logbook, StationBook};
use crate::i18n::{t, tf, tp};

/// 统计口径的持久化 key（视图偏好，不进备份正文）。
const SCOPE_KEY: &str = "log-stats-scope";

#[component]
pub(super) fn LogStatsPanel(
  logbook: RwSignal<Logbook>,
  book: RwSignal<StationBook>,
) -> impl IntoView {
  // 统计口径（全部 / 某个台站 / 某个操作员）：切页也记着，俱乐部台就不用每次重选。
  let scope = RwSignal::new(crate::util::storage::get(SCOPE_KEY).unwrap_or_default());
  let set_scope = Callback::new(move |key: String| {
    crate::util::storage::set(SCOPE_KEY, &key);
    scope.set(key);
  });

  view! {
    {move || {
      let all = logbook.get().entries;
      if all.is_empty() {
        return view! { <div></div> }.into_any();
      }
      let parsed = StatsScope::from_key(&scope.get());
      let entries = book.with(|b| scoped_entries(&all, b, &parsed));
      // 口径选择器的选项：台站档案 + 日志里出现过的操作员（都没有就不显示这一行）。
      let profiles = book.with(|b| {
        b.profiles
          .iter()
          .map(|p| (p.id, station_title(p)))
          .collect::<Vec<_>>()
      });
      let ops = book.with(|b| operators(&all, b));
      // 网格地图的「本台」：按口径里的台站算，口径是操作员时用当前台站。
      let my_grid = book.with(|b| {
        b.resolve(parsed.station.unwrap_or(0))
          .gridsquare
          .clone()
      });
      // 计数 / 去重 / 排序都在 core（`log_stats::overview`）：这里只做视图。
      let LogOverview {
        total,
        callsigns: n_calls,
        dxcc,
        grids,
        modes,
        bands,
        confirmed,
      } = overview(&entries);
      let n_dxcc = dxcc.len();
      let pending = ham_web_core::logbook::pending_qsl(&entries);
      view! {
        {(profiles.len() > 1 || ops.len() > 1)
          .then(|| {
            // 选项带分组（台站 / 操作员两个面），与原生 `optgroup` 对应。
            let mut options: Vec<SelectOption> = Vec::new();
            for (id, title) in &profiles {
              options.push(SelectOption::grouped(
                t("log.stats-scope-by-station"),
                StatsScope::of_station(*id).key(),
                title.clone(),
              ));
            }
            for op in &ops {
              options.push(SelectOption::grouped(
                t("log.stats-scope-by-operator"),
                StatsScope::of_operator(op).key(),
                op.clone(),
              ));
            }
            view! {
              <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span>{move || t("log.stats-scope")}</span>
                <NativeSelect
                  value=Signal::derive(move || scope.get())
                  on_change=set_scope
                  options=options
                  // 「全部」这个说法词典里已经有了（`exam.all`）：中文释义全库唯一，
                  // 同义的 key 只能复用一个，不能各造一份。
                  placeholder=Signal::derive(move || t("exam.all"))
                  aria_label=Signal::derive(move || t("log.stats-scope"))
                  class="w-40"
                />
                {(!parsed.is_all())
                  .then(|| {
                    view! {
                      <span>{tf("log.stats-scope-count", &[&entries.len().to_string()])}</span>
                    }
                  })}
              </div>
            }
          })}
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("log.total-qsos") value=move || total />
          <Stat label=t("log.unique-callsigns") value=move || n_calls />
          <Stat label=t("log.dxcc-entities") value=move || n_dxcc />
          <Stat label=t("log.qsl-confirmed") value=move || confirmed />
        </div>
        <section class="grid gap-4 sm:grid-cols-2">
          <div class="rounded-xl border bg-card p-4">
            <h3 class="mb-3 text-sm font-semibold">{move || t("log.by-mode")}</h3>
            <BarList items=modes />
          </div>
          <div class="rounded-xl border bg-card p-4">
            <h3 class="mb-3 text-sm font-semibold">{move || t("log.by-band")}</h3>
            <BarList items=bands />
          </div>
        </section>
        <section class="rounded-xl border bg-card p-4">
          <h3 class="mb-2 text-sm font-semibold">
            {move || t("log.dxcc-entities")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tp("log.worked-2", n_dxcc, &[&n_dxcc.to_string()])}
            </span>
          </h3>
          {if dxcc.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("log.no-recognized-entities-yet")}</div> }.into_any()
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
            {move || t("log.qsls-pending")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tp("common.cards-to-chase", pending.len(), &[&pending.len().to_string()])}
            </span>
          </h3>
          {if pending.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("log.all-sent-qsls-are")}</div> }.into_any()
          } else {
            view! {
              <div class="divide-y">
                {pending
                  .iter()
                  .map(|e| {
                    let via = if e.qsl_sent {
                      t("knowledge.paper")
                    } else if e.lotw_sent {
                      "LoTW".to_owned()
                    } else {
                      "eQSL".to_owned()
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
            {move || t("learning.grids-worked")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {tf("log.entry", &[&grids.len().to_string()])}
            </span>
          </h3>
          {if grids.is_empty() {
            view! { <div class="text-xs text-muted-foreground">{move || t("log.no-grids-yet-they")}</div> }.into_any()
          } else {
            view! {
              <div class="space-y-3">
                <GridMap entries=entries.clone() station_grid=my_grid.clone() />
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
