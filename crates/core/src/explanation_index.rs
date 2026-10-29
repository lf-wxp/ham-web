//! 题库当前版本的解析索引：让存档快照里的解析跟上题库。
//!
//! 错题本、考后复盘会把题目存一份快照（错题还要靠快照在题库换版后继续复习），快照里的
//! 解析是**存档当时**的版本。解析订正后（`data/explanations.json` → 题库），快照不会自己
//! 更新，界面上就可能还在展示已经被改掉的旧文字。
//!
//! 约定：**展示解析时以题库当前版本为准，快照只在题库里认不出这道题时兜底。**
//! 本模块只负责「怎么认出是同一道题」和「该显示哪份解析」，从不改写快照 —— 题干 / 选项 /
//! 答案是错题 key（内容指纹）的输入，不能跟着动。
//!
//! 认题方式：稳定 ID **且**题干、选项、答案集合都一致。只看 ID 不够：解析表以内容指纹为
//! key，正是因为题库重排 / 增删后同一个 ID 可能指向另一道题（见 [`crate::fingerprint`]）；
//! 只看 ID + 题干也不够：勘误只改了答案或选项时，旧答案配上针对新答案的解析会自相矛盾。
//! 对不上就沿用快照自己的解析 —— 宁可旧，也不能把别的题（或别的答案）的解析安上来。
//! 空白按 [`collapse_whitespace`] 规整，与内容指纹同口径。

use std::collections::HashMap;

use crate::bank::Bank;
use crate::exam_review::ReviewItem;
use crate::question::QuestionItem;
use crate::text::collapse_whitespace;

/// 「是不是同一道题」的比较口径：题干 + 选项（`A. 文本` 形式）+ 排序后的答案。
///
/// 选项用考后复盘快照里的 `A. 文本` 形式，这样 [`ReviewItem`]（只存了字符串）也能比。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity {
  stem: String,
  options: Vec<String>,
  answer: Vec<String>,
}

impl Identity {
  fn new<'a>(
    stem: &str,
    options: impl IntoIterator<Item = String>,
    answer: impl IntoIterator<Item = &'a String>,
  ) -> Self {
    let mut answer: Vec<String> = answer.into_iter().cloned().collect();
    answer.sort_unstable();
    Self {
      stem: collapse_whitespace(stem),
      options: options
        .into_iter()
        .map(|o| collapse_whitespace(&o))
        .collect(),
      answer,
    }
  }

  fn of_question(q: &QuestionItem) -> Self {
    Self::new(
      &q.question,
      q.options.iter().map(|o| format!("{}. {}", o.key, o.text)),
      &q.answer_keys,
    )
  }

  fn of_review(item: &ReviewItem) -> Self {
    Self::new(&item.question, item.options.iter().cloned(), &item.answer)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
  identity: Identity,
  /// 题库里没有解析的记为空串。
  explanation: String,
}

/// 题库当前版本的解析索引，按稳定 ID 查。
///
/// 可以一个题库一个题库地并入（[`ExplanationIndex::extend`]）：页面不必等齐三个题库，
/// 哪个先到哪个先生效，认不出的题照旧沿用快照。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplanationIndex {
  by_id: HashMap<String, Entry>,
}

impl ExplanationIndex {
  /// 由题库题目建索引。
  #[must_use]
  pub fn from_questions<'a>(questions: impl IntoIterator<Item = &'a QuestionItem>) -> Self {
    let mut index = Self::default();
    index.extend(questions);
    index
  }

  /// 并入一批题库题目。同一个 ID 出现多次时先到先得（A / B / C 三库的 ID 本就不重复）。
  pub fn extend<'a>(&mut self, questions: impl IntoIterator<Item = &'a QuestionItem>) {
    for q in questions {
      let Some(id) = q.stable_id() else {
        continue;
      };
      self.by_id.entry(id).or_insert_with(|| Entry {
        identity: Identity::of_question(q),
        explanation: q.explanation.clone().unwrap_or_default(),
      });
    }
  }

  /// 还没并入任何题。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.by_id.is_empty()
  }

  /// 题库里认得出就返回它的解析（没有解析为空串），认不出为 `None`。
  fn find(&self, id: &str, identity: impl FnOnce() -> Identity) -> Option<&str> {
    let entry = self.by_id.get(id)?;
    (entry.identity == identity()).then_some(entry.explanation.as_str())
  }

  /// 题目快照该显示的解析：题库里认得出就以题库为准（包括题库已撤下解析的情况，此时为
  /// `None`，不再留着旧文字），认不出则用快照自己的。
  #[must_use]
  pub fn question_explanation(&self, q: &QuestionItem) -> Option<String> {
    let latest = q
      .stable_id()
      .and_then(|id| self.find(&id, || Identity::of_question(q)));
    match latest {
      Some(latest) => (!latest.is_empty()).then(|| latest.to_owned()),
      None => q.explanation.clone(),
    }
  }

  /// 考后复盘某一项该显示的解析：规则同 [`ExplanationIndex::question_explanation`]，
  /// 快照里的空串视为没有解析。
  #[must_use]
  pub fn review_explanation(&self, item: &ReviewItem) -> Option<String> {
    let text = self
      .find(&item.id, || Identity::of_review(item))
      .unwrap_or(&item.explanation);
    (!text.is_empty()).then(|| text.to_owned())
  }
}

/// 这批快照的稳定 ID 涉及哪些题库（去重，按 A / B / C 顺序）。
///
/// 只加载这些题库就够认出快照里的题：错题快照与复盘快照的 ID 来自题库本身，前缀决定
/// 所属题库（见 [`Bank::of_id`]）。没有快照时返回空，调用方就不必去拉任何题库。
#[must_use]
pub fn banks_for<'a>(ids: impl IntoIterator<Item = Option<&'a str>>) -> Vec<Bank> {
  let mut seen = [false; Bank::ALL.len()];
  for id in ids {
    let bank = Bank::of_id(id);
    if let Some(i) = Bank::ALL.iter().position(|b| *b == bank) {
      seen[i] = true;
    }
  }
  Bank::ALL
    .into_iter()
    .zip(seen)
    .filter_map(|(bank, hit)| hit.then_some(bank))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionOption, QuestionType};

  fn q(id: Option<&str>, stem: &str, explanation: Option<&str>) -> QuestionItem {
    QuestionItem {
      id: id.map(Into::into),
      codes: Codes { j: None, p: None },
      question: stem.into(),
      options: vec![
        QuestionOption {
          key: "A".into(),
          text: "是".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "否".into(),
        },
      ],
      answer_keys: vec!["A".into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: explanation.map(Into::into),
    }
  }

  fn review(id: &str, stem: &str, explanation: &str) -> ReviewItem {
    ReviewItem {
      id: id.into(),
      code: String::new(),
      question: stem.into(),
      options: vec!["A. 是".into(), "B. 否".into()],
      answer: vec!["A".into()],
      given: vec![],
      explanation: explanation.into(),
      category: "1.1.1".into(),
    }
  }

  fn index(bank: &[QuestionItem]) -> ExplanationIndex {
    ExplanationIndex::from_questions(bank)
  }

  #[test]
  fn stale_snapshot_follows_the_bank() {
    let idx = index(&[q(Some("A-1"), "题干", Some("新解析"))]);
    let snap = q(Some("A-1"), "题干", Some("旧解析"));
    assert_eq!(idx.question_explanation(&snap).as_deref(), Some("新解析"));
    // 只读：快照本身不会被改。
    assert_eq!(snap.explanation.as_deref(), Some("旧解析"));
  }

  #[test]
  fn snapshot_without_explanation_gets_the_banks() {
    let idx = index(&[q(Some("A-1"), "题干", Some("补上的解析"))]);
    let snap = q(Some("A-1"), "题干", None);
    assert_eq!(
      idx.question_explanation(&snap).as_deref(),
      Some("补上的解析")
    );
  }

  #[test]
  fn same_id_but_different_stem_is_not_the_same_question() {
    // 题库重排后 `A-1` 指向了另一道题：不能把它的解析安到旧题上。
    let idx = index(&[q(Some("A-1"), "新题干", Some("别的题的解析"))]);
    let snap = q(Some("A-1"), "旧题干", Some("自己的解析"));
    assert_eq!(
      idx.question_explanation(&snap).as_deref(),
      Some("自己的解析")
    );
  }

  #[test]
  fn corrected_answer_or_options_are_not_the_same_question() {
    // 勘误只改了答案：旧答案不能配上针对新答案的解析。
    let mut fixed = q(Some("A-1"), "题干", Some("针对新答案的解析"));
    fixed.answer_keys = vec!["B".into()];
    let idx = index(&[fixed]);
    let snap = q(Some("A-1"), "题干", Some("针对旧答案的解析"));
    assert_eq!(
      idx.question_explanation(&snap).as_deref(),
      Some("针对旧答案的解析")
    );

    // 勘误只改了选项文字，同理。
    let mut reworded = q(Some("A-1"), "题干", Some("新解析"));
    reworded.options[0].text = "对".into();
    let idx = index(&[reworded]);
    assert_eq!(
      idx.question_explanation(&snap).as_deref(),
      Some("针对旧答案的解析")
    );
  }

  #[test]
  fn answer_order_and_whitespace_do_not_matter() {
    let mut bank = q(Some("A-1"), "题  干", Some("新"));
    bank.answer_keys = vec!["B".into(), "A".into()];
    let idx = index(&[bank]);
    let mut snap = q(Some("A-1"), "题 干", Some("旧"));
    snap.answer_keys = vec!["A".into(), "B".into()];
    assert_eq!(idx.question_explanation(&snap).as_deref(), Some("新"));
  }

  #[test]
  fn unknown_or_idless_snapshot_keeps_its_own() {
    let idx = index(&[q(Some("A-1"), "题干", Some("新"))]);
    let gone = q(Some("A-999"), "题干", Some("旧"));
    assert_eq!(idx.question_explanation(&gone).as_deref(), Some("旧"));
    let idless = q(None, "题干", Some("旧"));
    assert_eq!(idx.question_explanation(&idless).as_deref(), Some("旧"));
    // 快照本来就没有解析：认不出时仍是没有。
    let bare = q(Some("A-999"), "题干", None);
    assert_eq!(idx.question_explanation(&bare), None);
  }

  #[test]
  fn withdrawn_explanation_is_not_kept_from_the_snapshot() {
    // 题库把这题的解析撤下了（比如发现写错），就不该继续展示快照里的旧文字。
    let idx = index(&[q(Some("A-1"), "题干", None)]);
    let snap = q(Some("A-1"), "题干", Some("写错的旧解析"));
    assert_eq!(idx.question_explanation(&snap), None);
  }

  #[test]
  fn j_code_is_the_fallback_identity() {
    let mut bank = q(None, "题干", Some("新"));
    bank.codes.j = Some("LY0001".into());
    let idx = index(&[bank]);
    let mut snap = q(None, "题干", Some("旧"));
    snap.codes.j = Some("LY0001".into());
    assert_eq!(idx.question_explanation(&snap).as_deref(), Some("新"));
  }

  #[test]
  fn first_occurrence_of_an_id_wins() {
    let idx = index(&[
      q(Some("A-1"), "题干", Some("先到的")),
      q(Some("A-1"), "题干", Some("后到的")),
    ]);
    let snap = q(Some("A-1"), "题干", None);
    assert_eq!(idx.question_explanation(&snap).as_deref(), Some("先到的"));
  }

  #[test]
  fn banks_can_arrive_one_by_one() {
    let mut idx = ExplanationIndex::default();
    assert!(idx.is_empty());
    let a = q(Some("A-1"), "甲题", Some("甲的新解析"));
    let b = q(Some("B-1"), "乙题", Some("乙的新解析"));
    let snap_a = q(Some("A-1"), "甲题", Some("旧"));
    let snap_b = q(Some("B-1"), "乙题", Some("旧"));

    idx.extend(&[a]);
    assert_eq!(
      idx.question_explanation(&snap_a).as_deref(),
      Some("甲的新解析")
    );
    // B 库还没到：沿用快照。
    assert_eq!(idx.question_explanation(&snap_b).as_deref(), Some("旧"));

    idx.extend(&[b]);
    assert_eq!(
      idx.question_explanation(&snap_b).as_deref(),
      Some("乙的新解析")
    );
  }

  #[test]
  fn review_item_prefers_the_bank_and_falls_back_to_its_own() {
    let idx = index(&[q(Some("A-1"), "题干", Some("新解析"))]);
    assert_eq!(
      idx
        .review_explanation(&review("A-1", "题干", "旧解析"))
        .as_deref(),
      Some("新解析")
    );
    // 题干对不上 / ID 不在：用存档时的。
    assert_eq!(
      idx
        .review_explanation(&review("A-1", "别的题干", "旧解析"))
        .as_deref(),
      Some("旧解析")
    );
    assert_eq!(
      idx
        .review_explanation(&review("A-9", "题干", "旧解析"))
        .as_deref(),
      Some("旧解析")
    );
    // 空索引（题库还没加载 / 加载失败）：全部用存档时的。
    assert_eq!(
      ExplanationIndex::default()
        .review_explanation(&review("A-1", "题干", "旧解析"))
        .as_deref(),
      Some("旧解析")
    );
  }

  #[test]
  fn review_item_with_changed_answer_or_options_keeps_its_own() {
    let mut fixed = q(Some("A-1"), "题干", Some("新解析"));
    fixed.answer_keys = vec!["B".into()];
    let idx = index(&[fixed]);
    assert_eq!(
      idx
        .review_explanation(&review("A-1", "题干", "旧解析"))
        .as_deref(),
      Some("旧解析")
    );

    let idx = index(&[q(Some("A-1"), "题干", Some("新解析"))]);
    let mut item = review("A-1", "题干", "旧解析");
    item.options = vec!["A. 对".into(), "B. 否".into()];
    assert_eq!(idx.review_explanation(&item).as_deref(), Some("旧解析"));
  }

  #[test]
  fn review_empty_explanations_read_as_none() {
    // 题库撤下了解析：不再展示存档里的。
    let idx = index(&[q(Some("A-1"), "题干", None)]);
    assert_eq!(
      idx.review_explanation(&review("A-1", "题干", "旧解析")),
      None
    );
    // 认不出且存档里本来就是空串：没有解析。
    assert_eq!(
      ExplanationIndex::default().review_explanation(&review("A-1", "题干", "")),
      None
    );
  }

  #[test]
  fn banks_for_follows_id_prefixes_in_order_without_duplicates() {
    assert_eq!(banks_for([]), Vec::<Bank>::new());
    assert_eq!(
      banks_for([Some("C-3"), Some("A-1"), Some("C-9")]),
      vec![Bank::A, Bank::C]
    );
    assert_eq!(banks_for([Some("B-2")]), vec![Bank::B]);
    // 没有 ID 时按 `Bank::of_id` 的口径算 A 库。
    assert_eq!(banks_for([None]), vec![Bank::A]);
  }
}
