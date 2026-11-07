//! 通联日志统计概览：总数 / 不同呼号 / DXCC / QSL 确认，模式与波段分布；
//! 首屏以下的 DXCC 实体、奖项、待回卡与网格地图见 [`super::log_stats_detail`]。

use leptos::prelude::*;

use ham_web_core::log_stats::{LogOverview, overview};

use crate::components::common::Deferred;
use crate::ui::Stat;

use super::bar_list::BarList;
use super::log_stats_detail::LogStatsDetail;
use super::stats_scope_select::{StatsScopeSelect, scoped_log_entries, use_stats_scope};
use super::{Logbook, StationBook};
use crate::i18n::t;

#[component]
pub(super) fn LogStatsPanel(
  logbook: RwSignal<Logbook>,
  book: RwSignal<StationBook>,
) -> impl IntoView {
  // 统计口径（全部 / 某个台站 / 某个操作员）：切页也记着，俱乐部台就不用每次重选。
  let scope = use_stats_scope();
  let scoped = scoped_log_entries(scope);
  // 计数 / 去重 / 排序都在 core（`log_stats::overview`）：这里只做视图。
  // 挂成 Memo 由概览与下面的明细共用，借用读取 —— 此前 `scoped.get()` 先把整份日志克隆一遍再算。
  let summary = Memo::new(move |_| scoped.with(|entries| overview(entries)));
  // 奖状申请表与待回卡「显示全部」的展开态放在闭包外：面板随日志变化整体重建，放在里面会被收起。
  let apply_open = RwSignal::new(false);
  let show_all_pending = RwSignal::new(false);

  view! {
    {move || {
      if logbook.with(|lb| lb.entries.is_empty()) {
        return view! { <div></div> }.into_any();
      }
      let LogOverview { total, callsigns: n_calls, dxcc, modes, bands, confirmed, .. } = summary.get();
      let n_dxcc = dxcc.len();
      view! {
        <StatsScopeSelect scope=scope count=Signal::derive(move || scoped.with(Vec::len)) />
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=Signal::derive(move || t("log.total-qsos")) value=move || total />
          <Stat label=Signal::derive(move || t("log.unique-callsigns")) value=move || n_calls />
          <Stat label=Signal::derive(move || t("log.dxcc-entities")) value=move || n_dxcc />
          <Stat label=Signal::derive(move || t("log.qsl-confirmed")) value=move || confirmed />
        </div>
        <section class="grid gap-4 sm:grid-cols-2">
          <div class=crate::kit::card_pad_class()>
            <h3 class="mb-3 text-sm">{move || t("log.by-mode")}</h3>
            <BarList items=modes />
          </div>
          <div class=crate::kit::card_pad_class()>
            <h3 class="mb-3 text-sm">{move || t("log.by-band")}</h3>
            <BarList items=bands />
          </div>
        </section>
      }
      .into_any()
    }}
    // 首屏以下：实体清单、奖项进度、待回卡、网格地图（含一张世界地图）。重度用户（几千条日志）
    // 这一段同步建完要几百毫秒，推到首帧之后（见 `Deferred`）。放在上面的响应式闭包**外面**：
    // 放在里面的话每记一条通联整块都会重新「延后一帧」，明细闪一下、页面跳一下。
    // 占位 52rem：有日志时明细 = 实体清单（≥ 5rem）+ 奖状进度（≥ 49rem，各皮肤 / 宽度实测）再加待回卡与
    // 网格地图（有数据时各 50rem 上下），取最稳的下限；日志为空时明细整块不出现，不占位。
    <Deferred min_height=Signal::derive(move || {
      if logbook.with(|lb| lb.entries.is_empty()) { String::new() } else { "52rem".to_owned() }
    })>
      <LogStatsDetail
        logbook=logbook
        scoped=scoped
        summary=summary
        scope=scope
        book=book
        apply_open=apply_open
        show_all_pending=show_all_pending
      />
    </Deferred>
  }
}
