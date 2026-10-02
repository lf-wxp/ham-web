//! 练习 / 考试进度与偏好的本地持久化（key 与旧版 Next.js 实现保持兼容）。
//!
//! 收藏 / 分组 / 笔记属于**高频读取**的集合：`is_bookmarked()` 在渲染题目列表时对
//! 每题调用一次，若每次都穿透到 `localStorage` 做一次 JSON 反序列化，一次渲染就是
//! N 次全表解析。这里为这三个集合加进程内缓存（读穿透、写回写），并监听其它标签页
//! 的 `storage` 事件失效缓存（见 [`install_cross_tab_sync`]）。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use ham_web_core::Bank;
use ham_web_core::QuestionItem;
use ham_web_core::bookmark_groups::BookmarkGroups;
use ham_web_core::exam_review::ExamReview;
use ham_web_core::practice::PracticeOrder;
use ham_web_core::saved_state::{ExamSavedState, PracticeSavedState, keys};

use crate::util::{now_ms, storage};

/// 收藏集合的存储 key。
const BOOKMARKS_KEY: &str = "bookmarks";
/// 收藏分组的存储 key。
const GROUPS_KEY: &str = "bookmark-groups";

/// 受缓存保护的集合。
///
/// 字段为 `None` 表示尚未加载（读穿透时会填充）。
#[derive(Default)]
struct Caches {
  bookmarks: Option<HashSet<String>>,
  groups: Option<BookmarkGroups>,
  notes: Option<HashMap<String, String>>,
}

thread_local! {
  static CACHES: RefCell<Caches> = RefCell::new(Caches::default());
}

/// 在缓存上执行 `f`（不可变借用）。
fn with_caches<R>(f: impl FnOnce(&mut Caches) -> R) -> R {
  CACHES.with(|c| f(&mut c.borrow_mut()))
}

/// 丢弃全部缓存（其它标签页清空了 `localStorage` 时调用）。
fn invalidate_all() {
  with_caches(|c| *c = Caches::default());
}

/// 某个 key 变更后失效对应缓存。返回是否命中受缓存的 key。
fn invalidate(key: &str) -> bool {
  match key {
    BOOKMARKS_KEY => with_caches(|c| c.bookmarks = None),
    GROUPS_KEY => with_caches(|c| c.groups = None),
    NOTES_KEY => with_caches(|c| c.notes = None),
    _ => return false,
  }
  true
}

/// 监听其它标签页对本源 `localStorage` 的写入，命中受缓存的 key 时失效对应缓存。
///
/// 不监听的话，两个标签页同时打开时会各自持有过期的内存副本并互相覆盖。
/// 只需在应用启动时调用一次；回调注册为全局常驻（与 Service Worker 监听同理）。
pub fn install_cross_tab_sync() {
  use wasm_bindgen::JsCast;
  use wasm_bindgen::closure::Closure;

  let cb = Closure::wrap(Box::new(move |ev: web_sys::StorageEvent| match ev.key() {
    // `localStorage.clear()` 时 `key` 为 null，此时全部失效。
    Some(k) => {
      let _ = invalidate(&k);
    }
    None => invalidate_all(),
  }) as Box<dyn FnMut(web_sys::StorageEvent)>);

  let _ =
    crate::util::window().add_event_listener_with_callback("storage", cb.as_ref().unchecked_ref());
  // 全局常驻监听，故意不回收。
  cb.forget();
}

pub fn load_practice(bank: Bank, version: Option<&str>) -> Option<PracticeSavedState> {
  storage::get_json::<PracticeSavedState>(&PracticeSavedState::storage_key(bank, version))
    .filter(|s| s.is_supported())
}

pub fn save_practice(state: &PracticeSavedState) {
  storage::set_json(
    &PracticeSavedState::storage_key(state.bank, state.version_id.as_deref()),
    state,
  );
}

pub fn clear_practice(bank: Bank, version: Option<&str>) {
  storage::remove(&PracticeSavedState::storage_key(bank, version));
}

pub fn load_last_mode() -> Option<PracticeOrder> {
  storage::get(keys::PRACTICE_LAST_MODE).and_then(|s| PracticeOrder::parse(&s))
}

pub fn save_last_mode(mode: PracticeOrder) {
  storage::set(keys::PRACTICE_LAST_MODE, mode.as_str());
}

pub fn load_no_resume(bank: Bank, version: Option<&str>) -> bool {
  storage::get(&keys::no_resume(bank, version)).as_deref() == Some("1")
}

pub fn save_no_resume(bank: Bank, version: Option<&str>, value: bool) {
  let key = keys::no_resume(bank, version);
  if value {
    storage::set(&key, "1");
  } else {
    storage::remove(&key);
  }
}

pub fn load_exam(bank: Bank, version: Option<&str>) -> Option<ExamSavedState> {
  storage::get_json::<ExamSavedState>(&ExamSavedState::storage_key(bank, version))
    .filter(|s| s.is_valid(now_ms()))
}

pub fn save_exam(state: &ExamSavedState) {
  storage::set_json(
    &ExamSavedState::storage_key(state.bank, state.version_id.as_deref()),
    state,
  );
}

pub fn clear_exam(bank: Bank, version: Option<&str>) {
  storage::remove(&ExamSavedState::storage_key(bank, version));
}

/// 快捷键说明是否已展示过；未展示时标记为已展示并返回 `false`。
pub fn help_seen(key: &str) -> bool {
  storage::get(key).as_deref() == Some("1")
}

pub fn mark_help_seen(key: &str) {
  storage::set(key, "1");
}

/// 收藏的题目（stable_id 集合）。
pub fn load_bookmarks() -> HashSet<String> {
  with_caches(|c| {
    c.bookmarks
      .get_or_insert_with(|| storage::get_json(BOOKMARKS_KEY).unwrap_or_default())
      .clone()
  })
}

/// 在缓存的收藏集合上执行 `f`，避免整表克隆。
fn with_bookmarks<R>(f: impl FnOnce(&HashSet<String>) -> R) -> R {
  with_caches(|c| {
    f(c
      .bookmarks
      .get_or_insert_with(|| storage::get_json(BOOKMARKS_KEY).unwrap_or_default()))
  })
}

/// 把缓存中的收藏集合写回 `localStorage`。
fn persist_bookmarks() {
  with_caches(|c| {
    if let Some(set) = &c.bookmarks {
      storage::set_json(BOOKMARKS_KEY, set);
    }
  });
}

/// 切换收藏状态，返回切换后是否已收藏。
pub fn toggle_bookmark(id: &str) -> bool {
  let added = with_caches(|c| {
    let set = c
      .bookmarks
      .get_or_insert_with(|| storage::get_json(BOOKMARKS_KEY).unwrap_or_default());
    if set.contains(id) {
      set.remove(id);
      false
    } else {
      set.insert(id.to_owned());
      true
    }
  });
  persist_bookmarks();
  // 取消收藏时同步清理各分组里的残留，避免「幽灵」分组条目。
  if !added {
    with_caches(|c| {
      // 同时借用收藏与分组两个字段：`prune` 只需要读收藏集合，克隆整表再传进去
      // 会让一次取消变成 O(收藏数) 的拷贝。
      let Caches {
        bookmarks,
        groups,
        notes: _,
      } = c;
      let set =
        bookmarks.get_or_insert_with(|| storage::get_json(BOOKMARKS_KEY).unwrap_or_default());
      let g = groups.get_or_insert_with(|| storage::get_json(GROUPS_KEY).unwrap_or_default());
      g.prune(set);
    });
    persist_groups();
  }
  added
}

/// 读取收藏分组。
pub fn load_groups() -> BookmarkGroups {
  with_caches(|c| {
    c.groups
      .get_or_insert_with(|| storage::get_json(GROUPS_KEY).unwrap_or_default())
      .clone()
  })
}

/// 把缓存中的收藏分组写回 `localStorage`。
fn persist_groups() {
  with_caches(|c| {
    if let Some(groups) = &c.groups {
      storage::set_json(GROUPS_KEY, groups);
    }
  });
}

/// 保存收藏分组。
pub fn save_groups(groups: &BookmarkGroups) {
  with_caches(|c| c.groups = Some(groups.clone()));
  persist_groups();
}

/// 题目私人笔记（key = stable_id）。
const NOTES_KEY: &str = "question-notes";

/// 读取某题的笔记。
pub fn load_note(id: &str) -> Option<String> {
  with_caches(|c| {
    c.notes
      .get_or_insert_with(|| storage::get_json(NOTES_KEY).unwrap_or_default())
      .get(id)
      .cloned()
  })
}

/// 保存某题的笔记（空文本则删除）。
pub fn save_note(id: &str, text: &str) {
  // 改与写在同一次借用里完成：分两次会把「已改内存、未落盘」的中间状态暴露出去，
  // 也多一次 `borrow_mut`。
  with_caches(|c| {
    let notes = c
      .notes
      .get_or_insert_with(|| storage::get_json(NOTES_KEY).unwrap_or_default());
    if text.trim().is_empty() {
      notes.remove(id);
    } else {
      notes.insert(id.to_owned(), text.to_owned());
    }
    storage::set_json(NOTES_KEY, notes);
  });
}

/// 题目是否已收藏。
pub fn is_bookmarked(id: &str) -> bool {
  with_bookmarks(|set| set.contains(id))
}

/// 最近一次模拟考试的逐题复盘快照。
///
/// 只存最近一次：逐题快照约几十 KB，若按成绩历史那样保留 50 份会撑满本地存储。
const EXAM_REVIEW_KEY: &str = "exam:last-review";

/// 读取最近一次考试的复盘快照。
pub fn load_exam_review() -> Option<ExamReview> {
  storage::get_json(EXAM_REVIEW_KEY)
}

/// 保存最近一次考试的复盘快照（覆盖式）。
pub fn save_exam_review(review: &ExamReview) {
  storage::set_json(EXAM_REVIEW_KEY, review);
}

/// 模拟考试交卷后暂存的「本次错题」列表（供练习页一键重练）。
const WRONG_EXAM_PREFIX: &str = "exam:wrong";

fn wrong_exam_key(bank: Bank) -> String {
  format!("{WRONG_EXAM_PREFIX}:{bank}")
}

/// 暂存本次模拟考试的错题。
pub fn save_wrong_exam(bank: Bank, questions: &[QuestionItem]) {
  storage::set_json(&wrong_exam_key(bank), &questions.to_vec());
}

/// 读取暂存的错题列表。
pub fn load_wrong_exam(bank: Bank) -> Option<Vec<QuestionItem>> {
  storage::get_json(&wrong_exam_key(bank))
}

/// 清除暂存的错题列表。
pub fn clear_wrong_exam(bank: Bank) {
  storage::remove(&wrong_exam_key(bank));
}
