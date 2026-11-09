//! 日志派生统计的会话级缓存：日志变了才重算，切页回来直接复用。
//!
//! 日志页、统计页、DXCC 地图都要「按统计口径过滤后的日志」及其上的概览、奖项进度、DXCC 申请表、
//! 体检报告。这些结果只取决于日志内容与口径，此前却每进一次页面从头算一遍（重度用户 5000 条
//! 日志，奖项进度一项在手机上就是几十毫秒，日志页和统计页各算一次）。
//!
//! 做法：把它们挂成 [`LogStore`](super::LogStore) 里的 `Memo`。`Memo` 是惰性的 —— 没人读就不算 ——
//! 订阅着 `logbook`，日志一变只是标脏，下一次有人读时才重算；而 store 在应用根上创建，
//! 离开页面不会释放。所以第一次进页面照常算，之后再进就是直接取值。

use ham_web_core::award_apply::{ApplyRow, dxcc_record_sheet};
use ham_web_core::award_progress::AwardProgress;
use ham_web_core::log_health::{HealthContext, HealthReport, IssueKind, check};
use ham_web_core::log_stats::{LogOverview, StatsScope, overview, scoped_entries};
use leptos::prelude::*;

use super::{LogEntry, Logbook, StationBook};

/// 统计口径的持久化 key（视图偏好，不进备份正文）。
pub(super) const SCOPE_KEY: &str = "log-stats-scope";

/// 本机时区相对 UTC 的偏移（分钟，东八区 = 480）。
///
/// `getTimezoneOffset()` 返回的是「本地比 UTC 晚多少分钟」（东八区是 -480），取负才是偏移。
fn local_offset_minutes() -> i32 {
  -js_sys::Date::new_0().get_timezone_offset() as i32
}

/// 当前 UTC 时间（自 1970-01-01 起经过的分钟数）。
pub(super) fn now_utc_minutes() -> i64 {
  (js_sys::Date::now() / 60_000.0).floor() as i64
}

/// 按统计口径过滤后的通联（跟随日志、台站档案与口径变化）。
///
/// 上层统计都吃 `&[LogEntry]`，这里克隆一份子集交给它们，见 [`scoped_entries`]。
pub(super) fn build_scoped(
  logbook: RwSignal<Logbook>,
  station: RwSignal<StationBook>,
  scope: RwSignal<String>,
) -> Memo<Vec<LogEntry>> {
  Memo::new(move |_| {
    let raw = StatsScope::from_key(&scope.get());
    logbook.with(|lb| {
      station.with(|b| {
        // 持久化的口径可能指向已删的台站 / 已不存在的操作员，先剔掉失效片段（见 `live`）。
        let parsed = raw.live(&lb.entries, b);
        scoped_entries(&lb.entries, b, &parsed)
      })
    })
  })
}

/// 日志上的派生结果，见模块文档。
///
/// **`overview` / `awards` / `dxcc_sheet` 一律按共享口径 [`Self::scope`] 算。** 页面要显示这些结果时，
/// 口径必须来自 `use_stats_scope()`（它返回的就是 `scope`），并用 `scoped_log_entries(scope)` 取过滤后的日志 ——
/// 那个函数认得共享口径，直接给出这里的 [`Self::scoped`]。页面若想按自己的另一个口径算，
/// 不能读这里的结果（口径对不上，数字会静默地不跟着变），只能对自建的 `scoped_log_entries(自己的信号)` 现算。
#[derive(Clone, Copy)]
pub struct LogDerived {
  /// 统计口径（短串，见 `StatsScope::key`）：全站统计页共用这一个信号，
  /// 在一处选了台站 / 操作员，别处照样按它算。
  pub scope: RwSignal<String>,
  /// 按口径过滤后的通联。
  pub scoped: Memo<Vec<LogEntry>>,
  /// 日志页顶部的概览数字与分布。
  pub overview: Memo<LogOverview>,
  /// DXCC / WAZ / WAC / VUCC 等奖状进度。
  pub awards: Memo<AwardProgress>,
  /// DXCC 申请记录表（每个实体一条代表通联）。
  pub dxcc_sheet: Memo<Vec<ApplyRow>>,
  /// 体检报告，连同算出它时的时间（分钟）：见 [`Self::health_report`]。
  health: Memo<(HealthReport, i64)>,
  /// 只取报告部分的映射，构造时建一份、各处共用（`health_report` 每次调用不再新建）。
  report: Memo<HealthReport>,
  /// 手动让体检重算的计数。
  health_epoch: RwSignal<u32>,
}

impl LogDerived {
  pub(super) fn new(logbook: RwSignal<Logbook>, station: RwSignal<StationBook>) -> Self {
    let scope = RwSignal::new(crate::util::storage::get(SCOPE_KEY).unwrap_or_default());
    let scoped = build_scoped(logbook, station, scope);
    let health_epoch = RwSignal::new(0u32);
    let health = Memo::new(move |_| {
      health_epoch.track();
      // 「现在」取算的那一刻：体检只在日志变化时重算，不会自己定时刷新。
      let ctx = HealthContext {
        now_utc_minutes: now_utc_minutes(),
        local_offset_minutes: local_offset_minutes(),
      };
      (
        logbook.with(|lb| check(&lb.entries, &ctx)),
        ctx.now_utc_minutes,
      )
    });
    Self {
      scope,
      scoped,
      overview: Memo::new(move |_| scoped.with(|e| overview(e))),
      awards: Memo::new(move |_| scoped.with(|e| AwardProgress::from_entries(e))),
      dxcc_sheet: Memo::new(move |_| scoped.with(|e| dxcc_record_sheet(e))),
      health,
      // 只有体检结果变了才重新克隆；重算后报告没变（只是时间戳变了）时，下游不会被通知。
      report: Memo::new(move |_| health.with(|(report, _)| report.clone())),
      health_epoch,
    }
  }

  /// 体检报告（跟随日志）。
  ///
  /// 报告里有「时间在未来」的条目时，缓存的结果可能已经过期（算的时候还没到、现在到了），
  /// 读之前先判一下，过期就让它重算一次；没有这类条目则不会因时间流逝而变，直接复用。
  /// 要在组件体里调用（挂载时判一次），不要放进 Memo / Effect 里：判定过程会写 `health_epoch`。
  pub fn health_report(&self) -> Memo<HealthReport> {
    self.refresh_health_if_stale();
    self.report
  }

  /// 缓存的体检结果是否已被时间流逝作废；作废了就让它重算一次。
  fn refresh_health_if_stale(&self) {
    let stale = self.health.with_untracked(|(report, at)| {
      report.count(IssueKind::TimeInFuture) > 0 && now_utc_minutes() > *at
    });
    if stale {
      self.health_epoch.update(|n| *n += 1);
    }
  }
}
