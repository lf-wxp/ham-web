//! 成就解锁检测与全局提示：在作答 / 交卷等动作后检测新解锁的成就并弹出提示。

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
use crate::icons::{Icon, IconKind};
use crate::util::storage;

/// 已展示过的成就 id（避免每次刷新都重复提示）。
const SEEN_KEY: &str = "achievements-seen";

thread_local! {
  /// 本次会话新解锁、待展示的成就队列。
  static NEW: RwSignal<Vec<&'static Achievement>> = RwSignal::new(Vec::new());
}

/// 打卡状态（精简）。
#[derive(Deserialize, Default)]
struct Checkin {
  #[serde(default)]
  streak: usize,
}

/// 构造成就判定所需的统计摘要（题库覆盖率需异步加载，此处传空）。
fn build_stats() -> AchStats<'static> {
  let study = crate::study::load_stats();
  let checkin: Checkin = storage::get_json("daily-checkin").unwrap_or_default();
  let challenge = storage::get_json::<DailyResults>("daily-challenge").map_or(0, |r| r.days.len());
  let logbook: Logbook = storage::get_json("logbook").unwrap_or_default();

  let logged: HashSet<String> = logbook
    .entries
    .iter()
    .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
    .collect();
  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();
  let dxcc_count = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| done.iter().any(|x| x.as_str() == *p) || logged.contains(*p))
    .count();

  let history = crate::exam_history::load();
  let exam_passed = history
    .iter()
    .any(|r| !r.weak && r.correct >= ExamRule::of(Bank::from_param(Some(&r.bank))).pass);
  let exam_perfect = history
    .iter()
    .any(|r| !r.weak && r.total > 0 && r.correct == r.total);
  let grid_count = logbook
    .entries
    .iter()
    .filter(|e| !e.gridsquare.is_empty())
    .map(|e| e.gridsquare.clone())
    .collect::<HashSet<_>>()
    .len();
  let contest_count = logbook
    .entries
    .iter()
    .filter(|e| !e.contest_id.is_empty())
    .count();

  AchStats {
    total_correct: study.total.correct,
    coverages: &[],
    streak: checkin.streak,
    exam_passed,
    exam_perfect,
    challenge_count: challenge,
    dxcc_count,
    log_count: logbook.entries.len(),
    grid_count,
    contest_count,
    bookmarks: crate::store::load_bookmarks().len(),
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
  let pending = NEW.with(|s| *s);
  let timer: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);

  let dismiss = move || {
    pending.update(|v| {
      if !v.is_empty() {
        v.remove(0);
      }
    });
  };

  Effect::new(move |_| {
    if !pending.get().is_empty() {
      if let Some(h) = timer.get_value() {
        h.clear();
      }
      let h = set_timeout_with_handle(
        move || {
          pending.update(|v| {
            if !v.is_empty() {
              v.remove(0);
            }
          });
        },
        Duration::from_millis(3500),
      )
      .ok();
      timer.set_value(h);
    }
  });

  on_cleanup(move || {
    if let Some(Some(h)) = timer.try_get_value() {
      h.clear();
    }
  });

  move || {
    pending.get().first().copied().map(|a| {
      view! {
        <div
          role="status"
          aria-live="polite"
          class="fixed bottom-4 right-4 z-50 flex w-72 items-start gap-3 rounded-lg border bg-popover p-3 shadow-lg"
        >
          <div class="text-2xl leading-none">{a.icon}</div>
          <div class="min-w-0 flex-1">
            <div class="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              {t("解锁成就")}
            </div>
            <div class="mt-0.5 text-sm font-semibold">{a.name}</div>
            <div class="mt-0.5 text-[11px] leading-snug text-muted-foreground">{a.desc}</div>
          </div>
          <button
            type="button"
            class="opacity-60 transition-opacity hover:opacity-100"
            aria-label=t("关闭提示")
            on:click=move |_| dismiss()
          >
            <Icon kind=IconKind::X class="h-3.5 w-3.5" />
          </button>
        </div>
      }
      .into_any()
    })
  }
}
