//! 练习 / 考试进度与偏好的本地持久化（key 与旧版 Next.js 实现保持兼容）。

use std::collections::{HashMap, HashSet};

use ham_web_core::Bank;
use ham_web_core::QuestionItem;
use ham_web_core::bookmark_groups::BookmarkGroups;
use ham_web_core::practice::PracticeOrder;
use ham_web_core::saved_state::{ExamSavedState, PracticeSavedState, keys};

use crate::util::{now_ms, storage};

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
  storage::get_json("bookmarks").unwrap_or_default()
}

/// 切换收藏状态，返回切换后是否已收藏。
pub fn toggle_bookmark(id: &str) -> bool {
  let mut set = load_bookmarks();
  let added = if set.contains(id) {
    set.remove(id);
    false
  } else {
    set.insert(id.to_owned());
    true
  };
  storage::set_json("bookmarks", &set);
  // 取消收藏时同步清理各分组里的残留，避免「幽灵」分组条目。
  if !added {
    let mut groups = load_groups();
    groups.prune(&set);
    save_groups(&groups);
  }
  added
}

/// 读取收藏分组。
pub fn load_groups() -> BookmarkGroups {
  storage::get_json("bookmark-groups").unwrap_or_default()
}

/// 保存收藏分组。
pub fn save_groups(groups: &BookmarkGroups) {
  storage::set_json("bookmark-groups", groups);
}

/// 题目私人笔记（key = stable_id）。
const NOTES_KEY: &str = "question-notes";

/// 读取全部题目笔记。
pub fn load_notes() -> HashMap<String, String> {
  storage::get_json(NOTES_KEY).unwrap_or_default()
}

/// 读取某题的笔记。
pub fn load_note(id: &str) -> Option<String> {
  load_notes().get(id).cloned()
}

/// 保存某题的笔记（空文本则删除）。
pub fn save_note(id: &str, text: &str) {
  let mut notes = load_notes();
  if text.trim().is_empty() {
    notes.remove(id);
  } else {
    notes.insert(id.to_owned(), text.to_owned());
  }
  storage::set_json(NOTES_KEY, &notes);
}

/// 题目是否已收藏。
pub fn is_bookmarked(id: &str) -> bool {
  load_bookmarks().contains(id)
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
