use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ham_web_core::Bank;
use ham_web_core::QuestionItem;
use ham_web_core::exam::shuffle_in_place;
use ham_web_core::practice::{PracticeOrder, unique_to_bank};
use leptos::prelude::*;

use crate::data::{self, AppError, Questions};
use crate::util::random;

#[derive(Clone, Copy)]
pub(super) struct PracticeStore {
  /// 原始题库（顺序）。
  pub(super) all: RwSignal<Questions>,
  /// 当前题序：`all` 的下标。
  pub(super) order_idx: RwSignal<Vec<usize>>,
  pub(super) index: RwSignal<usize>,
  pub(super) answers: RwSignal<HashMap<String, Vec<String>>>,
  pub(super) order: RwSignal<PracticeOrder>,
  pub(super) show_answer: RwSignal<bool>,
  pub(super) show_explanation: RwSignal<bool>,
  pub(super) loading: RwSignal<bool>,
  /// 本轮已计入错题本 / 统计的作答 key。
  pub(super) recorded: StoredValue<HashSet<String>>,
}

impl PracticeStore {
  pub(super) fn new() -> Self {
    Self {
      all: RwSignal::new(Arc::new(Vec::new())),
      order_idx: RwSignal::new(Vec::new()),
      index: RwSignal::new(0),
      answers: RwSignal::new(HashMap::new()),
      order: RwSignal::new(PracticeOrder::Sequential),
      show_answer: RwSignal::new(true),
      show_explanation: RwSignal::new(true),
      loading: RwSignal::new(true),
      recorded: StoredValue::new(HashSet::new()),
    }
  }

  /// 把当前题的作答计入错题本与统计；多选题可能反复修改，所以在离开题目时才计入，每题每轮只计一次。
  pub(super) fn commit_current(self) {
    let Some(pos) = self.index.try_get_untracked() else {
      return;
    };
    let Some(q) = self
      .order_idx
      .try_with_untracked(|o| o.get(pos).copied())
      .flatten()
      .and_then(|i| self.all.try_with_untracked(|a| a.get(i).cloned()).flatten())
    else {
      return;
    };
    let key = q.answer_key(pos);
    let Some(ans) = self
      .answers
      .try_with_untracked(|a| a.get(&key).cloned())
      .flatten()
      .filter(|a| !a.is_empty())
    else {
      return;
    };
    if self.recorded.try_update_value(|r| r.insert(key)) == Some(true) {
      crate::study::record_answer(&q, &ans);
    }
  }

  pub(super) fn reset(self) {
    self.commit_current();
    self.recorded.set_value(HashSet::new());
    self.all.set(Arc::new(Vec::new()));
    self.order_idx.set(Vec::new());
    self.index.set(0);
    self.answers.set(HashMap::new());
    self.order.set(PracticeOrder::Sequential);
    self.show_answer.set(true);
    self.show_explanation.set(true);
    self.loading.set(true);
  }

  pub(super) fn load(self, qs: Questions) {
    self.reset();
    self.order_idx.set((0..qs.len()).collect());
    self.all.set(qs);
    self.loading.set(false);
  }

  pub(super) fn set_order(self, order: PracticeOrder) {
    self.commit_current();
    self.recorded.set_value(HashSet::new());
    let mut idx: Vec<usize> = (0..self.all.with_untracked(|a| a.len())).collect();
    if order == PracticeOrder::Random {
      let mut rng = random;
      shuffle_in_place(&mut idx, &mut rng);
    }
    self.order.set(order);
    self.order_idx.set(idx);
    self.index.set(0);
    self.answers.set(HashMap::new());
  }

  pub(super) fn len(self) -> usize {
    self.order_idx.with(Vec::len)
  }

  pub(super) fn next(self) {
    self.commit_current();
    let max = self.len().saturating_sub(1);
    self.index.update(|i| *i = (*i + 1).min(max));
  }

  pub(super) fn prev(self) {
    self.commit_current();
    self.index.update(|i| *i = i.saturating_sub(1));
  }

  pub(super) fn jump(self, i: usize) {
    self.commit_current();
    self.index.set(i.min(self.len().saturating_sub(1)));
  }

  pub(super) fn question_at(self, pos: usize) -> Option<QuestionItem> {
    let idx = self.order_idx.with(|o| o.get(pos).copied())?;
    self.all.with(|a| a.get(idx).cloned())
  }

  pub(super) fn current(self) -> Option<(usize, QuestionItem)> {
    let i = self.index.get();
    self.question_at(i).map(|q| (i, q))
  }

  pub(super) fn current_key(self) -> Option<String> {
    let i = self.index.get();
    self.question_at(i).map(|q| q.answer_key(i))
  }

  /// 当前题序下的题目引用列表。
  pub(super) fn ordered<R>(self, f: impl FnOnce(&[&QuestionItem]) -> R) -> R {
    self.all.with(|all| {
      self
        .order_idx
        .with(|idx| f(&idx.iter().filter_map(|&i| all.get(i)).collect::<Vec<_>>()))
    })
  }
}

pub(super) async fn load_questions(
  version: Option<&str>,
  bank: Bank,
  unique: bool,
) -> Result<Questions, AppError> {
  if !unique {
    return data::load_bank(version, bank, true).await;
  }
  let a = data::load_bank(version, Bank::A, false).await?;
  let b = data::load_bank(version, Bank::B, false).await?;
  let c = data::load_bank(version, Bank::C, false).await?;
  Ok(Arc::new(unique_to_bank(bank, &a, &b, &c)))
}
