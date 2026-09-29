//! 练习 / 考试进度与偏好的本地持久化（key 与旧版 Next.js 实现保持兼容）。

use ham_web_core::Bank;
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
