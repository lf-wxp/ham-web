use ham_web_core::answer_card::{self, AnswerCardFilter, Answers, Cell, Flags, Jump};
use leptos::prelude::*;

use crate::data::Questions;
use crate::i18n::t;

/// 答题卡两套皮肤共用的响应式取数。
///
/// 像素版 `AnswerCardSheet` 与现代版 `ModernAnswerCard` 只负责画；「显示哪些格子、跳到哪一题、
/// 计数」都从这里取，判定本身在 core 的 `answer_card`（有单测）。以前两边各抄一份，
/// 靠注释约定同步，格子的无障碍名就已经分叉了。
#[derive(Clone, Copy)]
pub struct AnswerCardModel {
  pub questions: Signal<Questions>,
  pub answers: Signal<Answers>,
  pub flags: Signal<Flags>,
  pub finished: Signal<bool>,
  pub filter: Signal<AnswerCardFilter>,
  pub current_index: Signal<usize>,
}

impl AnswerCardModel {
  /// 当前筛选下要画的格子。**订阅**作答 / 标记 / 筛选 / 当前题 / 是否交卷，放在视图闭包里用。
  pub fn cells(&self) -> Vec<Cell> {
    let finished = self.finished.get();
    let filter = self.filter.get();
    let current = self.current_index.get();
    self.questions.with(|qs| {
      self.answers.with(|a| {
        self
          .flags
          .with(|f| answer_card::visible_cells(qs, a, f, filter, finished, current))
      })
    })
  }

  /// 总题数（订阅）。
  pub fn total(&self) -> usize {
    self.questions.with(|q| q.len())
  }

  /// 已作答题数（订阅）。
  pub fn answered_count(&self) -> usize {
    self.answers.with(answer_card::answered_count)
  }

  /// 已标记题数（订阅）。
  pub fn flagged_count(&self) -> usize {
    self.flags.with(answer_card::flagged_count)
  }

  /// 点击「跳到…」时算目标。**不订阅**：只在事件回调里读一次当时的状态。
  pub fn jump_target(&self, jump: Jump) -> Option<usize> {
    let current = self.current_index.get_untracked();
    self.questions.with_untracked(|qs| {
      self.answers.with_untracked(|a| {
        self
          .flags
          .with_untracked(|f| answer_card::jump_target(qs, a, f, jump, current))
      })
    })
  }
}

/// 格子的无障碍名：`base`（题号）+ 标记 / 对错状态，用「 · 」连接。
///
/// 状态只靠颜色或角标表达的话读屏听不到，所以两套皮肤都要把它写进名字里。
/// 可见文本仍然只是题号，`base` 必须包含它（label-in-name）。
pub fn cell_label(base: String, flagged: bool, verdict: Option<bool>) -> String {
  let mut parts = vec![base];
  if flagged {
    parts.push(t("exam.flagged"));
  }
  match verdict {
    Some(true) => parts.push(t("exam.correct-label")),
    Some(false) => parts.push(t("exam.wrong-label")),
    None => {}
  }
  parts.join(" · ")
}
