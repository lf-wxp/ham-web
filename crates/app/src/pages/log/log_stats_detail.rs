//! 日志统计面板的首屏以下部分：DXCC 实体、奖项进度、待回卡、网格地图。
//!
//! 由 [`super::log_stats_panel`] 包在 `Deferred` 里，新页面第一帧画出来之后才挂载。

use leptos::prelude::*;

use ham_web_core::log_stats::{LogOverview, StatsScope};

use super::awards_panel::AwardsPanel;
use super::grid_map::GridMap;
use super::pending_qsl_list::PendingQslList;
use super::{LogEntry, Logbook, StationBook};
use crate::i18n::{t, tf, tp};

#[component]
pub(super) fn LogStatsDetail(
  logbook: RwSignal<Logbook>,
  scoped: Memo<Vec<LogEntry>>,
  summary: Memo<LogOverview>,
  scope: RwSignal<String>,
  book: RwSignal<StationBook>,
  apply_open: RwSignal<bool>,
  show_all_pending: RwSignal<bool>,
) -> impl IntoView {
  move || {
    if logbook.with(|lb| lb.entries.is_empty()) {
      return view! { <div></div> }.into_any();
    }
    let parsed = StatsScope::from_key(&scope.get());
    // 网格地图的「本台」：按口径里的台站算，口径是操作员时用当前台站。
    let my_grid = book.with(|b| b.resolve(parsed.station.unwrap_or(0)).gridsquare.clone());
    let (dxcc, grids) = summary.with(|s| (s.dxcc.clone(), s.grids.clone()));
    let n_dxcc = dxcc.len();
    view! {
      <section class=crate::kit::card_pad_class()>
        <h3 class="mb-2 text-sm">
          {move || t("log.dxcc-entities")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {tp("log.worked-count", n_dxcc, &[&n_dxcc.to_string()])}
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
                    <span class=crate::kit::tile_class("px-2.5 py-0.5 text-xs")>{e}</span>
                  }
                })
                .collect_view()}
            </div>
          }
          .into_any()
        }}
      </section>
      <AwardsPanel entries=scoped apply_open=apply_open />
      <PendingQslList scoped=scoped show_all=show_all_pending />
      <section class=crate::kit::card_pad_class()>
        <h3 class="mb-2 text-sm">
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
              <GridMap entries=scoped.get() station_grid=my_grid.clone() />
              <div class="flex flex-wrap gap-1.5">
                {grids
                  .into_iter()
                  .map(|g| {
                    view! {
                      <span class=crate::kit::tile_class("px-2.5 py-0.5 font-mono text-xs")>{g}</span>
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
  }
}
