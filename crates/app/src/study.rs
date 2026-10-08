//! 错题本与累计答题统计的本地持久化，以及从旧版进度（练习 / 考试存档）的一次性迁移。

use std::cell::Cell;

use ham_web_core::mistake_book::{MistakeBook, RecordOutcome, StudyStats, question_key};
use ham_web_core::question_stats::QuestionStats;
use ham_web_core::study_plan::{DailyLog, StudyPlan};
use ham_web_core::{Bank, QuestionItem};

use crate::util::{local_today, now_ms, storage};
use crate::{data, store};

// 当前这道题开始作答的时间戳（毫秒）。单题作答入口在展示题目时调用
// `note_question_start`，随后 `record_answer` / `record_self_assess` 据此计算本题耗时。
thread_local! {
  static QUESTION_START: Cell<i64> = const { Cell::new(0) };
}

/// 标记「开始看一道题」；后续单题记录会自动计算从此刻起的耗时。
pub fn note_question_start() {
  QUESTION_START.with(|c| c.set(now_ms()));
}

/// 取出并清零当前题耗时（毫秒）；未标记时为 0。
fn take_duration() -> u64 {
  QUESTION_START.with(|c| {
    let start = c.replace(0);
    if start > 0 {
      (now_ms() - start).max(0) as u64
    } else {
      0
    }
  })
}

const BOOK_KEY: &str = "mistake-book";
const STATS_KEY: &str = "study-stats";
const DAILY_KEY: &str = "study-daily";
const PLAN_KEY: &str = "learning-plan";
const QSTATS_KEY: &str = "question-stats";
const SEEDED_KEY: &str = "mistake-book:seeded";
const SEEN_SEEDED_KEY: &str = "study-stats:seen-seeded";

pub fn load_book() -> MistakeBook {
  let mut book: MistakeBook = storage::get_json(BOOK_KEY).unwrap_or_default();
  // 题库排版归一化（2026-10）后历史 key 会失效：用题目快照重算 key 并归一化快照文本，
  // 幂等，因此每次加载都顺手做一次即可。
  if book.rekey() > 0 {
    save_book(&book);
  }
  book
}

fn save_book(book: &MistakeBook) {
  storage::set_json(BOOK_KEY, book);
}

pub fn load_stats() -> StudyStats {
  storage::get_json(STATS_KEY).unwrap_or_default()
}

fn save_stats(stats: &StudyStats) {
  storage::set_json(STATS_KEY, stats);
}

/// 加载单题作答统计。
pub fn load_qstats() -> QuestionStats {
  storage::get_json(QSTATS_KEY).unwrap_or_default()
}

fn save_qstats(stats: &QuestionStats) {
  storage::set_json(QSTATS_KEY, stats);
}

pub fn load_daily() -> DailyLog {
  storage::get_json(DAILY_KEY).unwrap_or_default()
}

pub fn load_plan() -> StudyPlan {
  storage::get_json(PLAN_KEY).unwrap_or_default()
}

pub fn save_plan(plan: &StudyPlan) {
  storage::set_json(PLAN_KEY, plan);
}

fn is_new(stats: &StudyStats, q: &QuestionItem) -> bool {
  !stats
    .bank(Bank::of_id(q.id_str()))
    .is_some_and(|b| b.has_seen(q))
}

/// 记录一次作答（空作答忽略），同时更新错题本与分类统计；耗时取当前题标记的时长。
pub fn record_answer(q: &QuestionItem, answer: &[String]) -> Option<RecordOutcome> {
  let dur = take_duration();
  record_many(std::iter::once((q, answer, dur)))
    .into_iter()
    .next()
}

/// 批量记录作答（模拟考试交卷），只读写一次存储；每题耗时由调用方传入。
pub fn record_many<'a>(
  items: impl IntoIterator<Item = (&'a QuestionItem, &'a [String], u64)>,
) -> Vec<RecordOutcome> {
  record_items(items, true)
}

/// `daily` 为假时不计入今日作答量（旧存档迁移）。
fn record_items<'a>(
  items: impl IntoIterator<Item = (&'a QuestionItem, &'a [String], u64)>,
  daily: bool,
) -> Vec<RecordOutcome> {
  let now = now_ms();
  let today = local_today();
  let mut book = load_book();
  let mut stats = load_stats();
  let mut qstats = load_qstats();
  let mut log = daily.then(load_daily);
  let outcomes: Vec<RecordOutcome> = items
    .into_iter()
    .filter_map(|(q, a, dur)| {
      let new = is_new(&stats, q);
      let outcome = book.record(q, a, now)?;
      let correct = q.is_answer_correct(a);
      stats.record(q, correct);
      qstats.record(&question_key(q), correct, now, dur);
      if let Some(log) = log.as_mut() {
        log.add(&today, new, dur);
      }
      Some(outcome)
    })
    .collect();
  if !outcomes.is_empty() {
    save_book(&book);
    save_stats(&stats);
    save_qstats(&qstats);
    if let Some(log) = &log {
      storage::set_json(DAILY_KEY, log);
    }
    crate::achievements::detect_new();
  }
  outcomes
}

/// 记录闪卡自评（会 / 不会）；耗时取当前题标记的时长。
pub fn record_self_assess(q: &QuestionItem, known: bool) {
  let dur = take_duration();
  let now = now_ms();
  let mut book = load_book();
  let mut stats = load_stats();
  let mut qstats = load_qstats();
  let mut log = load_daily();
  log.add(&local_today(), is_new(&stats, q), dur);
  book.record_result(q, known, &[], now);
  stats.record(q, known);
  qstats.record(&question_key(q), known, now, dur);
  save_book(&book);
  save_stats(&stats);
  save_qstats(&qstats);
  storage::set_json(DAILY_KEY, &log);
  crate::achievements::detect_new();
}

/// 手动移出一道错题。
pub fn remove_mistake(key: &str) {
  let mut book = load_book();
  book.remove(key);
  save_book(&book);
}

/// 标注错题错因（`cause` 为空则清除标注）。
pub fn set_mistake_cause(key: &str, cause: &str) {
  let mut book = load_book();
  if book.set_cause(key, cause) {
    save_book(&book);
  }
}

/// 清空错题本。
pub fn clear_book() {
  save_book(&MistakeBook::default());
}

/// 把旧版仅存在于练习 / 考试存档中的作答一次性合并进错题本与统计，并补记「做过的题」
/// 供题库覆盖率使用（应用启动时调用）。
///
/// 仍可恢复的考试存档会在交卷时正常记录，这里只补记做过、不计入错题与正确率，以免重复计数。
pub async fn ensure_seeded() {
  let need_book = storage::get(SEEDED_KEY).is_none();
  let need_seen = storage::get(SEEN_SEEDED_KEY).is_none();
  if !need_book && !need_seen {
    return;
  }
  storage::set(SEEDED_KEY, "1");
  storage::set(SEEN_SEEDED_KEY, "1");
  let now = now_ms();
  // (题目, 作答, 是否来自仍可恢复的考试)
  let mut collected: Vec<(QuestionItem, Vec<String>, bool)> = Vec::new();
  let mut collect =
    |qs: &[QuestionItem], order: &[usize], answers: &[Option<Vec<String>>], resumable: bool| {
      for (pos, ans) in answers.iter().enumerate() {
        if let Some(ans) = ans.as_ref().filter(|a| !a.is_empty())
          && let Some(q) = order.get(pos).and_then(|&i| qs.get(i))
        {
          collected.push((q.clone(), ans.clone(), resumable));
        }
      }
    };
  let versions = data::all_versions(false).await.unwrap_or_default();
  let mut version_ids: Vec<Option<String>> = vec![None];
  version_ids.extend(versions.into_iter().map(|v| Some(v.id)));
  for vid in version_ids {
    let vid = vid.as_deref();
    for bank in Bank::ALL {
      let practice = store::load_practice(bank, vid);
      let exam = store::load_exam(bank, vid);
      if practice.is_none() && exam.is_none() {
        continue;
      }
      let Ok(qs) = data::load_bank(vid, bank, false).await else {
        continue;
      };
      if let Some(state) = practice {
        collect(&qs, &state.order_indices, &state.answers_by_position, false);
      }
      if let Some(state) = exam {
        let rebuilt = state.reconstruct(&qs);
        let order: Vec<usize> = (0..rebuilt.len()).collect();
        collect(
          &rebuilt,
          &order,
          &state.answers_by_position,
          state.should_resume(now),
        );
      }
    }
  }
  if need_book {
    record_items(
      collected
        .iter()
        .filter(|(_, _, resumable)| !resumable)
        .map(|(q, a, _)| (q, a.as_slice(), 0)),
      false,
    );
  }
  if need_seen {
    let mut stats = load_stats();
    for (q, _, _) in &collected {
      stats.mark_seen(q);
    }
    save_stats(&stats);
  }
}

/// 每日学习提醒：应用打开期间每 30 秒检查一次，到达备考计划设定的提醒时间
/// （且当天尚未提醒过）时发浏览器通知。
pub fn start_study_reminder_watcher() {
  crate::util::request_notify_permission();
  leptos::prelude::set_interval(
    || {
      let plan = load_plan();
      let time = plan.reminder_time.trim().to_owned();
      if time.is_empty() {
        return;
      }
      let today = local_today();
      if storage::get("study-reminder-last").as_deref() == Some(today.as_str()) {
        return;
      }
      let now = js_sys::Date::new_0();
      let hhmm = format!("{:02}:{:02}", now.get_hours(), now.get_minutes());
      if hhmm == time {
        storage::set("study-reminder-last", &today);
        crate::util::notify(&crate::i18n::t("settings.time-to-study-today"));
      }
    },
    std::time::Duration::from_secs(30),
  );
}
