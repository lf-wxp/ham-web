//! 答题卡的筛选、跳转与格子状态。
//!
//! 两套皮肤（像素 `AnswerCardSheet`、现代 `ModernAnswerCard`）的画法完全不同，但「哪些题显示、
//! 跳到哪一题、一格是什么状态」必须是同一个答案。以前这些逻辑在两个文件里各写一份，靠文件头
//! 注释约定「改这边要同步那边」—— 结果两边格子的无障碍名已经分叉（像素版带标记 / 对错状态，
//! 现代版没有）。放在 core 里只有一份，也才测得到。

use std::collections::HashMap;

use crate::question::QuestionItem;

/// 作答记录：题目键（[`QuestionItem::answer_key`]）→ 选中的选项；空数组等同于未作答。
pub type Answers = HashMap<String, Vec<String>>;

/// 标记记录：题目键 → 是否标记（`false` 等同于没标记）。
pub type Flags = HashMap<String, bool>;

/// 答题卡筛选。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AnswerCardFilter {
  /// 全部题目。
  #[default]
  All,
  /// 只看未作答。
  Unanswered,
  /// 只看已标记。
  Flagged,
}

impl AnswerCardFilter {
  /// 写进本地存储的取值（`exam:answerCardFilter:{bank}`）。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::All => "all",
      Self::Unanswered => "unanswered",
      Self::Flagged => "flagged",
    }
  }

  /// [`Self::as_str`] 的逆；不认识的值返回 `None`，由调用方退回默认。
  #[must_use]
  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "all" => Some(Self::All),
      "unanswered" => Some(Self::Unanswered),
      "flagged" => Some(Self::Flagged),
      _ => None,
    }
  }

  /// 这一格在当前筛选下是否显示。
  #[must_use]
  pub const fn shows(self, answered: bool, flagged: bool) -> bool {
    match self {
      Self::All => true,
      Self::Unanswered => !answered,
      Self::Flagged => flagged,
    }
  }
}

/// 答题卡里的一格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
  /// 题目在整卷里的位置（从 0 起）。**筛选不会改它**：点格子跳转用的就是它。
  pub index: usize,
  /// 是否已作答。
  pub answered: bool,
  /// 是否被标记。
  pub flagged: bool,
  /// 是否是当前题。
  pub current: bool,
  /// 交卷后的对错；`None` 表示还在作答中。未作答的题交卷后算错。
  pub verdict: Option<bool>,
}

impl Cell {
  /// 显示用的题号（从 1 起）。
  #[must_use]
  pub const fn number(self) -> usize {
    self.index + 1
  }
}

/// 答题卡上的三种「跳到…」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jump {
  /// 从头数起的第一道未答题。
  FirstUnanswered,
  /// 当前题**之后**的下一道未答题。
  NextUnanswered,
  /// 当前题**之后**的下一道已标记题。
  NextFlagged,
}

fn is_answered(answers: &Answers, key: &str) -> bool {
  answers.get(key).is_some_and(|v| !v.is_empty())
}

fn is_flagged(flags: &Flags, key: &str) -> bool {
  flags.get(key).copied().unwrap_or(false)
}

/// 已作答题数（空选择不算）。
#[must_use]
pub fn answered_count(answers: &Answers) -> usize {
  answers.values().filter(|v| !v.is_empty()).count()
}

/// 已标记题数。
#[must_use]
pub fn flagged_count(flags: &Flags) -> usize {
  flags.values().filter(|&&v| v).count()
}

/// 当前筛选下要画出来的格子，按原题序排列。
///
/// `finished` 为真时每格带对错；`current` 是当前题的位置（越界也安全，只是没有格子是当前题）。
#[must_use]
pub fn visible_cells(
  questions: &[QuestionItem],
  answers: &Answers,
  flags: &Flags,
  filter: AnswerCardFilter,
  finished: bool,
  current: usize,
) -> Vec<Cell> {
  questions
    .iter()
    .enumerate()
    .filter_map(|(index, q)| {
      let key = q.answer_key(index);
      let selected = answers.get(&key).map(Vec::as_slice).unwrap_or_default();
      let answered = !selected.is_empty();
      let flagged = is_flagged(flags, &key);
      filter.shows(answered, flagged).then(|| Cell {
        index,
        answered,
        flagged,
        current: index == current,
        verdict: finished.then(|| q.is_answer_correct(selected)),
      })
    })
    .collect()
}

/// 跳转目标；找不到返回 `None`（调用方什么都不做）。
///
/// 「下一道」只往后找、**不回绕**：已经在最后一道未答题上再点，按钮不动，
/// 而不是悄悄把人带回卷首。
#[must_use]
pub fn jump_target(
  questions: &[QuestionItem],
  answers: &Answers,
  flags: &Flags,
  jump: Jump,
  current: usize,
) -> Option<usize> {
  let start = match jump {
    Jump::FirstUnanswered => 0,
    Jump::NextUnanswered | Jump::NextFlagged => current.saturating_add(1),
  };
  questions
    .iter()
    .enumerate()
    .skip(start)
    .find(|&(index, q)| {
      let key = q.answer_key(index);
      match jump {
        Jump::FirstUnanswered | Jump::NextUnanswered => !is_answered(answers, &key),
        Jump::NextFlagged => is_flagged(flags, &key),
      }
    })
    .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionType};

  fn q(id: Option<&str>) -> QuestionItem {
    QuestionItem {
      id: id.map(str::to_owned),
      codes: Codes::default(),
      question: "q".into(),
      options: vec![],
      answer_keys: vec!["A".into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  /// 五道题：A-0 答对、A-1 答错、A-2 选了又清空（等同未答）、A-3 标记未答、A-4 未答。
  fn fixture() -> (Vec<QuestionItem>, Answers, Flags) {
    let qs: Vec<_> = (0..5).map(|i| q(Some(&format!("A-{i}")))).collect();
    let answers = Answers::from([
      ("A-0".into(), vec!["A".into()]),
      ("A-1".into(), vec!["B".into()]),
      ("A-2".into(), vec![]),
    ]);
    let flags = Flags::from([("A-3".into(), true), ("A-1".into(), false)]);
    (qs, answers, flags)
  }

  fn indices(cells: &[Cell]) -> Vec<usize> {
    cells.iter().map(|c| c.index).collect()
  }

  #[test]
  fn filter_round_trips_and_rejects_unknown() {
    for f in [
      AnswerCardFilter::All,
      AnswerCardFilter::Unanswered,
      AnswerCardFilter::Flagged,
    ] {
      assert_eq!(AnswerCardFilter::parse(f.as_str()), Some(f));
    }
    assert_eq!(AnswerCardFilter::parse(""), None);
    assert_eq!(
      AnswerCardFilter::parse("ALL"),
      None,
      "区分大小写：存储里只会有 as_str 写出的值"
    );
    assert_eq!(AnswerCardFilter::default(), AnswerCardFilter::All);
  }

  #[test]
  fn filter_truth_table() {
    use AnswerCardFilter::*;
    for (answered, flagged) in [(false, false), (false, true), (true, false), (true, true)] {
      assert!(All.shows(answered, flagged));
      assert_eq!(Unanswered.shows(answered, flagged), !answered);
      assert_eq!(Flagged.shows(answered, flagged), flagged);
    }
  }

  #[test]
  fn counts_ignore_empty_selections_and_false_flags() {
    let (_, answers, flags) = fixture();
    assert_eq!(answered_count(&answers), 2, "A-2 是空选择，不算已答");
    assert_eq!(flagged_count(&flags), 1, "A-1 的 false 不算标记");
    assert_eq!(answered_count(&Answers::new()), 0);
    assert_eq!(flagged_count(&Flags::new()), 0);
  }

  #[test]
  fn cells_follow_filter_and_keep_original_positions() {
    let (qs, answers, flags) = fixture();
    let cells = |f| visible_cells(&qs, &answers, &flags, f, false, 1);

    assert_eq!(indices(&cells(AnswerCardFilter::All)), [0, 1, 2, 3, 4]);
    // 筛选后 `index` 仍是整卷位置：点第 4 格跳到第 4 题，而不是「筛选结果里的第 4 个」。
    assert_eq!(indices(&cells(AnswerCardFilter::Unanswered)), [2, 3, 4]);
    assert_eq!(indices(&cells(AnswerCardFilter::Flagged)), [3]);
    assert_eq!(cells(AnswerCardFilter::Flagged)[0].number(), 4);

    let all = cells(AnswerCardFilter::All);
    assert!(all[0].answered && !all[2].answered);
    assert!(all[3].flagged && !all[1].flagged);
    assert_eq!(all.iter().filter(|c| c.current).count(), 1);
    assert!(all[1].current);
  }

  #[test]
  fn verdict_only_after_finishing_and_unanswered_counts_wrong() {
    let (qs, answers, flags) = fixture();
    let running = visible_cells(&qs, &answers, &flags, AnswerCardFilter::All, false, 0);
    assert!(running.iter().all(|c| c.verdict.is_none()));

    let done = visible_cells(&qs, &answers, &flags, AnswerCardFilter::All, true, 0);
    let verdicts: Vec<_> = done.iter().map(|c| c.verdict).collect();
    assert_eq!(
      verdicts,
      [
        Some(true),
        Some(false),
        Some(false),
        Some(false),
        Some(false)
      ],
      "答对 / 答错 / 清空 / 未答 / 未答：未答的题交卷后算错"
    );
  }

  #[test]
  fn out_of_range_current_is_safe() {
    let (qs, answers, flags) = fixture();
    let cells = visible_cells(
      &qs,
      &answers,
      &flags,
      AnswerCardFilter::All,
      false,
      usize::MAX,
    );
    assert_eq!(cells.len(), 5);
    assert!(cells.iter().all(|c| !c.current));
    assert!(visible_cells(&[], &answers, &flags, AnswerCardFilter::All, true, 0).is_empty());
  }

  #[test]
  fn questions_without_ids_are_keyed_by_position() {
    // 没有 id / J 码的题用 `pos:N` 做键（见 `QuestionItem::answer_key`）。
    let qs = vec![q(None), q(None)];
    let answers = Answers::from([("pos:1".into(), vec!["A".into()])]);
    let cells = visible_cells(
      &qs,
      &answers,
      &Flags::new(),
      AnswerCardFilter::Unanswered,
      false,
      0,
    );
    assert_eq!(indices(&cells), [0]);
  }

  #[test]
  fn jumps_go_forward_only_and_do_not_wrap() {
    let (qs, answers, flags) = fixture();
    let go = |jump, current| jump_target(&qs, &answers, &flags, jump, current);

    // 第一道未答题与当前位置无关；A-2 虽然有记录但是空选择，仍算未答。
    assert_eq!(go(Jump::FirstUnanswered, 0), Some(2));
    assert_eq!(go(Jump::FirstUnanswered, 4), Some(2));
    // 「下一道」严格在当前题之后。
    assert_eq!(go(Jump::NextUnanswered, 0), Some(2));
    assert_eq!(go(Jump::NextUnanswered, 2), Some(3));
    assert_eq!(go(Jump::NextUnanswered, 3), Some(4));
    assert_eq!(
      go(Jump::NextUnanswered, 4),
      None,
      "不回绕：最后一道之后没有了"
    );
    assert_eq!(go(Jump::NextFlagged, 0), Some(3));
    assert_eq!(go(Jump::NextFlagged, 3), None, "当前题自己不算「下一道」");
  }

  #[test]
  fn jumps_survive_degenerate_input() {
    let (qs, answers, flags) = fixture();
    assert_eq!(
      jump_target(&qs, &answers, &flags, Jump::NextUnanswered, usize::MAX),
      None
    );
    assert_eq!(
      jump_target(&[], &answers, &flags, Jump::FirstUnanswered, 0),
      None
    );

    let all_done: Answers = (0..5)
      .map(|i| (format!("A-{i}"), vec!["A".into()]))
      .collect();
    assert_eq!(
      jump_target(&qs, &all_done, &flags, Jump::FirstUnanswered, 0),
      None
    );
  }
}
