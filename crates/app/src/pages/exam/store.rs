use std::collections::HashMap;
use std::sync::Arc;

use ham_web_core::ExamRule;
use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::data::Questions;
use crate::util::now_ms;

#[derive(Clone, Copy)]
pub(super) struct ExamStore {
  pub(super) questions: RwSignal<Questions>,
  pub(super) answers: RwSignal<HashMap<String, Vec<String>>>,
  pub(super) flags: RwSignal<HashMap<String, bool>>,
  pub(super) index: RwSignal<usize>,
  pub(super) finished: RwSignal<bool>,
  pub(super) end_at: RwSignal<Option<i64>>,
}

impl ExamStore {
  pub(super) fn new() -> Self {
    Self {
      questions: RwSignal::new(Arc::new(Vec::new())),
      answers: RwSignal::new(HashMap::new()),
      flags: RwSignal::new(HashMap::new()),
      index: RwSignal::new(0),
      finished: RwSignal::new(false),
      end_at: RwSignal::new(None),
    }
  }

  pub(super) fn start(self, questions: Questions, rule: ExamRule) {
    self.questions.set(questions);
    self.answers.set(HashMap::new());
    self.flags.set(HashMap::new());
    self.index.set(0);
    self.finished.set(false);
    self.end_at.set(Some(now_ms() + rule.duration_ms()));
  }

  pub(super) fn reset(self) {
    self.questions.set(Arc::new(Vec::new()));
    self.answers.set(HashMap::new());
    self.flags.set(HashMap::new());
    self.index.set(0);
    self.finished.set(false);
    self.end_at.set(None);
  }

  pub(super) fn len(self) -> usize {
    self.questions.with(|q| q.len())
  }

  pub(super) fn next(self) {
    let max = self.len().saturating_sub(1);
    self.index.update(|i| *i = (*i + 1).min(max));
  }

  pub(super) fn prev(self) {
    self.index.update(|i| *i = i.saturating_sub(1));
  }

  pub(super) fn jump(self, i: usize) {
    self.index.set(i.min(self.len().saturating_sub(1)));
  }

  pub(super) fn current(self) -> Option<(usize, QuestionItem)> {
    let i = self.index.get();
    self.questions.with(|q| q.get(i).cloned().map(|q| (i, q)))
  }

  pub(super) fn current_key(self) -> Option<String> {
    let i = self.index.get();
    self.questions.with(|q| q.get(i).map(|q| q.answer_key(i)))
  }
}
