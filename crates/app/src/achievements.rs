//! 成就解锁检测与全局提示：在作答 / 交卷等动作后检测新解锁的成就并弹出提示。

use std::cell::Cell;
use std::collections::HashSet;
use std::time::Duration;

use ham_web_core::achievements::{ACHIEVEMENTS, AchStats, Achievement, unlocked};
use ham_web_core::daily_challenge::DailyResults;
use ham_web_core::logbook::Logbook;
use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;
use serde::Deserialize;

use crate::i18n::t;
use crate::icons::{Icon, IconKind, PixelSprite, badge_sprite};
use crate::util::storage;

/// 已展示过的成就 id（避免每次刷新都重复提示）。
const SEEN_KEY: &str = "achievements-seen";

thread_local! {
  /// 本次会话新解锁、待展示的成就队列。
  ///
  /// 用 `ArcRwSignal` 而非 `RwSignal`：`RwSignal` 是 arena 分配的，会挂到创建时所在的
  /// 响应式 `Owner` 上并随其清理而释放；这个队列是全站共享的，不能随某个页面 `Owner`
  /// 一起失效。引用计数信号只要还有引用就一直有效。
  static NEW: ArcRwSignal<Vec<&'static Achievement>> = ArcRwSignal::new(Vec::new());
  /// 日志摘要的 Memo（日志 store 在根组件里登记，见 [`track_logbook`]）。
  static LOG_SUMMARY: Cell<Option<Memo<LogSummary>>> = const { Cell::new(None) };
}

/// 打卡状态（精简）。
#[derive(Deserialize, Default)]
struct Checkin {
  #[serde(default)]
  streak: usize,
}

/// 成就判定里只依赖通联日志的那几项（进度页、成就页也用它，不必各自克隆整本日志再数一遍）。
#[derive(Clone, Default, PartialEq)]
pub(crate) struct LogSummary {
  /// 日志里出现过的「最想通联」前缀。
  pub(crate) logged: HashSet<String>,
  pub(crate) log_count: usize,
  pub(crate) grid_count: usize,
  pub(crate) contest_count: usize,
}

fn summarize(logbook: &Logbook) -> LogSummary {
  LogSummary {
    logged: logbook
      .entries
      .iter()
      .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
      .collect(),
    log_count: logbook.entries.len(),
    grid_count: logbook
      .entries
      .iter()
      .filter(|e| !e.gridsquare.is_empty())
      .map(|e| e.gridsquare.as_str())
      .collect::<HashSet<_>>()
      .len(),
    contest_count: logbook
      .entries
      .iter()
      .filter(|e| !e.contest_id.is_empty())
      .count(),
  }
}

/// 让成就检测读内存里的日志（与日志页同一份），摘要挂成 Memo：日志变了才重算。
///
/// 此前每答一题都从 `localStorage` 解析整本日志、逐条查前缀 —— 5000 条时这一步占了作答耗时的四成，
/// 而日志在刷题过程中根本不会变。由日志 store 在根组件里调用一次（`pages::log::provide_log_store`），
/// Memo 因此挂在根 Owner 上、与应用同寿。
pub fn track_logbook(logbook: RwSignal<Logbook>) {
  let memo = Memo::new(move |_| logbook.with(summarize));
  LOG_SUMMARY.with(|c| c.set(Some(memo)));
  // Memo 是惰性的：第一次读要把整本日志过一遍。空闲时先算好，别让它落在第一次作答上。
  crate::after_paint::when_idle(
    move || {
      let _ = memo.try_with_untracked(|_| ());
    },
    Duration::from_secs(10),
  );
}

/// 当前日志的摘要（来自日志 store 上的 Memo；store 还没登记时退回解析快照）。
pub(crate) fn log_summary() -> LogSummary {
  LOG_SUMMARY
    .with(Cell::get)
    .and_then(|m| m.try_get_untracked())
    .unwrap_or_else(|| summarize(&storage::get_json("logbook").unwrap_or_default()))
}

/// 日志里出现过的「最想通联」前缀（首页追台建议卡用；与成就判定同一份 Memo，不重算）。
pub fn logged_wanted_prefixes() -> HashSet<String> {
  log_summary().logged
}

/// 构造成就判定所需的统计摘要（题库覆盖率需异步加载，此处传空）。
fn build_stats() -> AchStats<'static> {
  let total_correct = crate::study::stats_shared().map_or(0, |s| s.total.correct);
  let checkin: Checkin = storage::get_json("daily-checkin").unwrap_or_default();
  let challenge = storage::get_json::<DailyResults>("daily-challenge").map_or(0, |r| r.days.len());
  let log = log_summary();

  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();
  let dxcc_count = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| done.iter().any(|x| x.as_str() == *p) || log.logged.contains(*p))
    .count();

  let history = crate::exam_history::load();
  let exam_passed = history
    .iter()
    .any(|r| !r.weak && r.correct >= ExamRule::of(Bank::from_param(Some(&r.bank))).pass);
  let exam_perfect = history
    .iter()
    .any(|r| !r.weak && r.total > 0 && r.correct == r.total);

  AchStats {
    total_correct,
    coverages: &[],
    streak: checkin.streak,
    exam_passed,
    exam_perfect,
    challenge_count: challenge,
    dxcc_count,
    log_count: log.log_count,
    grid_count: log.grid_count,
    contest_count: log.contest_count,
    bookmarks: crate::store::bookmark_count(),
  }
}

/// 当前已解锁的成就数（供学习报告等汇总展示）。
pub fn unlocked_count() -> usize {
  unlocked(&build_stats()).len()
}

/// 检测新解锁的成就，更新已见集合并把新解锁加入展示队列。
pub fn detect_new() {
  let current = unlocked(&build_stats());
  let mut seen: HashSet<String> = storage::get_json::<Vec<String>>(SEEN_KEY)
    .unwrap_or_default()
    .into_iter()
    .collect();

  let mut newly: Vec<&'static Achievement> = Vec::new();
  for id in current {
    if seen.insert(id.to_owned())
      && let Some(a) = ACHIEVEMENTS.iter().find(|a| a.id == id)
    {
      newly.push(a);
    }
  }
  if newly.is_empty() {
    return;
  }
  storage::set_json(SEEN_KEY, &seen.into_iter().collect::<Vec<_>>());
  NEW.with(|sig| sig.update(|v| v.extend(newly)));
}

/// 全局成就解锁提示：右下角弹出，逐条展示，自动消失。
#[component]
pub fn AchievementToast() -> impl IntoView {
  // `ArcRwSignal` 是 `Clone` 而非 `Copy`，被多个 `move` 闭包捕获时要显式 clone。
  let pending = NEW.with(|s| s.clone());
  // 退场态：置真后播退出动画，动画结束再真正移出队列。
  let leaving = RwSignal::new(false);
  let timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);
  // 退场动画播完后的「弹出队首」定时器：与 `timer` 一样需要能在队列变化时取消，
  // 否则退场期间新成就到达、`Effect` 重置 `leaving` 后，这个已排队的 pop 仍会提前弹出队首。
  let pop_timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);

  // 真正把队首移出队列（退场动画播完后调用）。
  let pop = {
    let pending = pending.clone();
    move || {
      pending.update(|v| {
        if !v.is_empty() {
          v.remove(0);
        }
      });
    }
  };

  // 触发退场：先置 leaving 播放退出动画，动画播完再 pop。加守卫避免退出期间重复触发。
  let dismiss = {
    let pop = pop.clone();
    move || {
      if !leaving.get_untracked() {
        leaving.set(true);
        let h = set_timeout_with_handle(
          pop.clone(),
          Duration::from_millis(crate::presence::exit_ms(crate::presence::ExitSpeed::Normal)),
        )
        .ok();
        pop_timer.set_value(h);
      }
    }
  };

  Effect::new({
    let pending = pending.clone();
    let dismiss = dismiss.clone();
    move |_| {
      // 队首变化（新成就入场或旧成就弹出）时重置退场态，让下一条从入场态重新开始。
      leaving.set(false);
      if !pending.get().is_empty() {
        if let Some(h) = timer.get_value() {
          h.clear();
        }
        if let Some(h) = pop_timer.get_value() {
          h.clear();
        }
        let h = set_timeout_with_handle(
          {
            let dismiss = dismiss.clone();
            move || dismiss()
          },
          Duration::from_millis(3500),
        )
        .ok();
        timer.set_value(h);
      }
    }
  });

  on_cleanup(move || {
    if let Some(Some(h)) = timer.try_get_value() {
      h.clear();
    }
    if let Some(Some(h)) = pop_timer.try_get_value() {
      h.clear();
    }
  });

  move || {
    // `dismiss` 捕获了非 `Copy` 的 `pending`，本身也不再 `Copy`；外层闭包是 `FnMut`，
    // 每次渲染都要重新 clone 一份交给事件处理器。
    let dismiss = dismiss.clone();
    pending.get().first().copied().map(|a| {
      view! {
        <div
          role="status"
          aria-live="polite"
          data-leaving=move || leaving.get().to_string()
          class="motion-toast pxl-popover fixed bottom-20 right-4 z-[var(--z-top)] flex w-72 items-start gap-3 p-3 xl:bottom-4"
        >
          // 徽章精灵是像素语言的一部分（20 枚手绘点阵图）。现代风没有精灵，
          // 用同一套线性图标里的奖杯顶上 —— 否则每次解锁成就都会在柔和卡片里
          // 弹出一块 3 倍放大的点阵图。
          {move || {
            if crate::kit::is_modern() {
              view! {
                <crate::kit_modern::ModernIcon
                  kind=crate::kit_modern::MIcon::Trophy
                  class="size-8 shrink-0"
                />
              }
                .into_any()
            } else {
              view! { <PixelSprite name=badge_sprite(a.id) scale=3 class="pxl-bob" /> }
                .into_any()
            }
          }}
          <div class="min-w-0 flex-1">
            <div class="pxl-label text-xs text-muted-foreground">
              {t("exam.achievement-unlocked")}
            </div>
            // 成就名与说明是中文原文：经 `t()` 的原文反查翻成当前语言（成就页同样这么写），
            // 直接渲染的话英文 / 西语界面的解锁提示会露出中文。
            <div class="mt-0.5 text-sm">{move || t(a.name)}</div>
            <div class="mt-0.5 text-xs leading-snug text-muted-foreground">{move || t(a.desc)}</div>
          </div>
          <button
            type="button"
            class="opacity-60 transition-opacity hover:opacity-100"
            aria-label=Signal::derive(move || t("learning.dismiss"))
            on:click=move |_| dismiss()
          >
            <Icon kind=IconKind::X class="size-6" />
          </button>
        </div>
      }
      .into_any()
    })
  }
}
