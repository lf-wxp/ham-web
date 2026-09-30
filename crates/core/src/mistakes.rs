//! 错题提取：从练习/考试进度中，比对作答与标准答案，得到答错的题目。

use crate::question::QuestionItem;

/// 一道错题。
#[derive(Debug, Clone, PartialEq)]
pub struct Mistake {
  /// 题目全文。
  pub question: QuestionItem,
  /// 用户作答。
  pub my_answer: Vec<String>,
  /// 正确答案。
  pub correct: Vec<String>,
}

/// 从一份进度（题序 + 按位置的作答）中提取错题。
///
/// `order_indices[i]` 是第 `i` 题对应的原始题库下标；`answers_by_position[i]` 是第 `i` 题的作答。
/// 跳过未作答（`None` 或空）的题。
#[must_use]
pub fn mistakes_from_state(
  questions: &[QuestionItem],
  order_indices: &[usize],
  answers_by_position: &[Option<Vec<String>>],
) -> Vec<Mistake> {
  answers_by_position
    .iter()
    .enumerate()
    .filter_map(|(pos, ans)| {
      let ans = ans.as_ref().filter(|a| !a.is_empty())?;
      let qi = order_indices.get(pos).and_then(|&i| questions.get(i))?;
      if qi.is_answer_correct(ans) {
        None
      } else {
        Some(Mistake {
          question: qi.clone(),
          my_answer: ans.clone(),
          correct: qi.answer_keys.clone(),
        })
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionType};

  fn q(id: &str, answer: &[&str]) -> QuestionItem {
    QuestionItem {
      id: Some(id.into()),
      codes: Codes::default(),
      question: format!("题目 {id}"),
      options: vec![],
      answer_keys: answer.iter().map(|s| s.to_string()).collect(),
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  #[test]
  fn extracts_only_wrong_answers() {
    let qs = vec![q("A-1", &["A"]), q("A-2", &["B"]), q("A-3", &["C", "D"])];
    let order = vec![0usize, 1, 2];
    let answers: Vec<Option<Vec<String>>> = vec![
      Some(vec!["A".into()]), // 对
      Some(vec!["A".into()]), // 错
      None,                   // 未答，跳过
    ];
    let m = mistakes_from_state(&qs, &order, &answers);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].question.id.as_deref(), Some("A-2"));
    assert_eq!(m[0].my_answer, vec!["A".to_owned()]);
    assert_eq!(m[0].correct, vec!["B".to_owned()]);
  }

  #[test]
  fn multi_select_ignores_order() {
    let qs = vec![q("A-1", &["A", "C"])];
    let order = vec![0usize];
    let answers = vec![Some(vec!["C".into(), "A".into()])];
    assert!(mistakes_from_state(&qs, &order, &answers).is_empty());
  }
}
