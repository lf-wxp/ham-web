//! 错题本与累计答题统计的本地持久化，以及从旧版进度（练习 / 考试存档）的一次性迁移。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
  /// 错题本：（原串，已 rekey 的解析结果），见 [`load_book`]。
  static BOOK: RefCell<Option<(String, Rc<MistakeBook>)>> = const { RefCell::new(None) };
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

/// 错题本（只读共享）。
///
/// 每答一题、每进一次错题本 / 首页都要读：重度用户（1000 条，带完整题目快照）每次解析 + rekey 在手机上
/// 是几十毫秒。所以缓存的是 **rekey 之后**的结果，原串和上次一样就直接返回（判据同
/// `storage::get_json_shared`：按原串比对，备份导入 / 跨标签页写入不会读到旧值）。
/// 要改错题本走 [`update_book`]，不要拿这份去 `clone` 再写回。
pub fn load_book() -> Rc<MistakeBook> {
  let raw = storage::get(BOOK_KEY);
  if let Some(raw) = raw.as_deref()
    && let Some(hit) = BOOK.with_borrow(|c| {
      c.as_ref()
        .filter(|(cached, _)| cached == raw)
        .map(|(_, book)| Rc::clone(book))
    })
  {
    return hit;
  }
  let mut book: MistakeBook = raw
    .as_deref()
    .and_then(|s| serde_json::from_str(s).ok())
    .unwrap_or_default();
  // 题库排版归一化（2026-10）后历史 key 会失效：用题目快照重算 key 并归一化快照文本，
  // 幂等，所以每次从原串解析出来都顺手做一次（命中缓存时已经做过）。
  if book.rekey() > 0 {
    return save_book(book);
  }
  let book = Rc::new(book);
  if let Some(raw) = raw {
    BOOK.with_borrow_mut(|c| *c = Some((raw, Rc::clone(&book))));
  }
  book
}

/// 改错题本：`f` 返回 `true` 表示改过，才整本写回；没改就原样放回缓存。
///
/// **契约：只要 `f` 动过错题本，就必须返回 `true`。** 返回 `false` 时这一轮的改动不会写回
/// （缓存也还是原串），等于静默丢数据。调试构建会对照改前的快照断言这一点，写错的调用在
/// 开发期就会 panic，而不是上线后丢记录。
///
/// 缓存里那份直接让出来给 `f` 改，免一次整本克隆。答对一道不在错题本里的题（最常见的情形）
/// 错题本不变，此前也要整本序列化写回一次 —— 1000 条时约 600 KB。
fn update_book(f: impl FnOnce(&mut MistakeBook) -> bool) {
  let shared = load_book();
  // 只有调试构建才付这一次整本克隆：用来核对「返回 false = 真没改」。
  #[cfg(debug_assertions)]
  let before = (*shared).clone();
  let raw = BOOK.with_borrow_mut(|c| c.take().map(|(raw, _)| raw));
  let mut book = Rc::unwrap_or_clone(shared);
  if f(&mut book) {
    save_book(book);
  } else {
    #[cfg(debug_assertions)]
    assert!(
      book == before,
      "[update_book] 闭包改了错题本却返回 false，这次改动不会被写回"
    );
    if let Some(raw) = raw {
      BOOK.with_borrow_mut(|c| *c = Some((raw, Rc::new(book))));
    }
  }
}

fn save_book(book: MistakeBook) -> Rc<MistakeBook> {
  let book = Rc::new(book);
  if let Ok(raw) = serde_json::to_string(&*book) {
    storage::set(BOOK_KEY, &raw);
    BOOK.with_borrow_mut(|c| *c = Some((raw, Rc::clone(&book))));
  }
  book
}

pub fn load_stats() -> StudyStats {
  stats_shared().map_or_else(StudyStats::default, Rc::unwrap_or_clone)
}

/// 学习统计（只读共享，不克隆；缓存见 `storage::get_json_shared`）。
pub fn stats_shared() -> Option<Rc<StudyStats>> {
  storage::get_json_shared(STATS_KEY)
}

fn save_stats(stats: StudyStats) {
  storage::set_json_shared(STATS_KEY, stats);
}

/// 加载单题作答统计。
pub fn load_qstats() -> QuestionStats {
  storage::get_json_shared(QSTATS_KEY).map_or_else(QuestionStats::default, Rc::unwrap_or_clone)
}

fn save_qstats(stats: QuestionStats) {
  storage::set_json_shared(QSTATS_KEY, stats);
}

pub fn load_daily() -> DailyLog {
  storage::get_json_shared(DAILY_KEY).map_or_else(DailyLog::default, Rc::unwrap_or_clone)
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
  // 计算变体（id 以 `variant-` 开头）只在当次练习里存在，不计入统计与错题本：
  // 官方内容指纹不能沾上本地生成的题目。
  if q
    .id_str()
    .is_some_and(|id| id.starts_with(ham_web_core::calc_variants::VARIANT_PREFIX))
  {
    return None;
  }
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
  // 统计类三份都「取出来改、改完写回」（`storage::take_json`）：缓存让出自己那份，不整份克隆。
  let mut stats: StudyStats = storage::take_json(STATS_KEY).unwrap_or_default();
  let mut qstats: QuestionStats = storage::take_json(QSTATS_KEY).unwrap_or_default();
  let mut log: Option<DailyLog> = daily.then(|| storage::take_json(DAILY_KEY).unwrap_or_default());
  let mut outcomes: Vec<RecordOutcome> = Vec::new();
  update_book(|book| {
    for (q, a, dur) in items {
      let new = is_new(&stats, q);
      let Some(outcome) = book.record(q, a, now) else {
        continue;
      };
      // 「掌握」= 把这只怪物打倒。放在这个汇总入口而不是各页面里记：练习页、考试、错题复习、
      // 回合战最终都走到这里，图鉴因此不会漏掉任何一条击败途径。
      if matches!(outcome, RecordOutcome::Mastered) {
        crate::rpg::defeat_monster(&question_key(q));
      }
      let correct = q.is_answer_correct(a);
      stats.record(q, correct);
      qstats.record(&question_key(q), correct, now, dur);
      if let Some(log) = log.as_mut() {
        log.add(&today, new, dur);
      }
      outcomes.push(outcome);
    }
    // 只有「答对且不在错题本里」时错题本不变：其余结果（答错 / 进度 / 掌握）都改了记录。
    outcomes
      .iter()
      .any(|o| !matches!(o, RecordOutcome::Correct))
  });
  if !outcomes.is_empty() {
    save_stats(stats);
    save_qstats(qstats);
    if let Some(log) = log {
      storage::set_json_shared(DAILY_KEY, log);
    }
    crate::achievements::detect_new();
  }
  outcomes
}

/// 记录闪卡自评（会 / 不会）；耗时取当前题标记的时长。
pub fn record_self_assess(q: &QuestionItem, known: bool) {
  let dur = take_duration();
  let now = now_ms();
  let mut stats: StudyStats = storage::take_json(STATS_KEY).unwrap_or_default();
  let mut qstats: QuestionStats = storage::take_json(QSTATS_KEY).unwrap_or_default();
  let mut log: DailyLog = storage::take_json(DAILY_KEY).unwrap_or_default();
  log.add(&local_today(), is_new(&stats, q), dur);
  update_book(|book| book.record_result(q, known, &[], now) != RecordOutcome::Correct);
  stats.record(q, known);
  qstats.record(&question_key(q), known, now, dur);
  save_stats(stats);
  save_qstats(qstats);
  storage::set_json_shared(DAILY_KEY, log);
  crate::achievements::detect_new();
}

/// 手动移出一道错题。
pub fn remove_mistake(key: &str) {
  update_book(|book| {
    let before = book.records.len();
    book.remove(key);
    book.records.len() != before
  });
}

/// 标注错题错因（`cause` 为空则清除标注）。
pub fn set_mistake_cause(key: &str, cause: &str) {
  update_book(|book| book.set_cause(key, cause));
}

/// 清空错题本。
pub fn clear_book() {
  save_book(MistakeBook::default());
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
    let mut stats: StudyStats = storage::take_json(STATS_KEY).unwrap_or_default();
    for (q, _, _) in &collected {
      stats.mark_seen(q);
    }
    save_stats(stats);
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
