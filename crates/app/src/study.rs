//! 错题本与累计答题统计的本地持久化，以及从旧版进度（练习 / 考试存档）的一次性迁移。

use ham_web_core::mistake_book::{MistakeBook, RecordOutcome, StudyStats};
use ham_web_core::study_plan::{DailyLog, StudyPlan};
use ham_web_core::{Bank, QuestionItem};

use crate::util::{local_today, now_ms, storage};
use crate::{data, store};

const BOOK_KEY: &str = "mistake-book";
const STATS_KEY: &str = "study-stats";
const DAILY_KEY: &str = "study-daily";
const PLAN_KEY: &str = "learning-plan";
const SEEDED_KEY: &str = "mistake-book:seeded";
const SEEN_SEEDED_KEY: &str = "study-stats:seen-seeded";

pub fn load_book() -> MistakeBook {
  storage::get_json(BOOK_KEY).unwrap_or_default()
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

/// 记录一次作答（空作答忽略），同时更新错题本与分类统计。
pub fn record_answer(q: &QuestionItem, answer: &[String]) -> Option<RecordOutcome> {
  record_many(std::iter::once((q, answer))).into_iter().next()
}

/// 批量记录作答（模拟考试交卷），只读写一次存储。
pub fn record_many<'a>(
  items: impl IntoIterator<Item = (&'a QuestionItem, &'a [String])>,
) -> Vec<RecordOutcome> {
  record_items(items, true)
}

/// `daily` 为假时不计入今日作答量（旧存档迁移）。
fn record_items<'a>(
  items: impl IntoIterator<Item = (&'a QuestionItem, &'a [String])>,
  daily: bool,
) -> Vec<RecordOutcome> {
  let now = now_ms();
  let today = local_today();
  let mut book = load_book();
  let mut stats = load_stats();
  let mut log = daily.then(load_daily);
  let outcomes: Vec<RecordOutcome> = items
    .into_iter()
    .filter_map(|(q, a)| {
      let new = is_new(&stats, q);
      let outcome = book.record(q, a, now)?;
      stats.record(q, q.is_answer_correct(a));
      if let Some(log) = log.as_mut() {
        log.add(&today, new);
      }
      Some(outcome)
    })
    .collect();
  if !outcomes.is_empty() {
    save_book(&book);
    save_stats(&stats);
    if let Some(log) = &log {
      storage::set_json(DAILY_KEY, log);
    }
  }
  outcomes
}

/// 记录闪卡自评（会 / 不会）。
pub fn record_self_assess(q: &QuestionItem, known: bool) {
  let mut book = load_book();
  let mut stats = load_stats();
  let mut log = load_daily();
  log.add(&local_today(), is_new(&stats, q));
  book.record_result(q, known, &[], now_ms());
  stats.record(q, known);
  save_book(&book);
  save_stats(&stats);
  storage::set_json(DAILY_KEY, &log);
}

/// 手动移出一道错题。
pub fn remove_mistake(key: &str) {
  let mut book = load_book();
  book.remove(key);
  save_book(&book);
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
        .map(|(q, a, _)| (q, a.as_slice())),
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
