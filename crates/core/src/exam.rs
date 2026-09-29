//! 考试规则、抽题与计分。

use crate::bank::Bank;
use crate::question::QuestionItem;

/// 某类考试的规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExamRule {
  /// 总题数。
  pub total: usize,
  /// 单选题数。
  pub singles: usize,
  /// 多选题数。
  pub multiples: usize,
  /// 限时（分钟）。
  pub minutes: u32,
  /// 合格线（答对题数）。
  pub pass: usize,
}

impl ExamRule {
  /// 获取题库对应规则。
  #[must_use]
  pub const fn of(bank: Bank) -> Self {
    match bank {
      Bank::A => Self {
        total: 40,
        singles: 32,
        multiples: 8,
        minutes: 40,
        pass: 30,
      },
      Bank::B => Self {
        total: 60,
        singles: 45,
        multiples: 15,
        minutes: 60,
        pass: 45,
      },
      Bank::C => Self {
        total: 90,
        singles: 70,
        multiples: 20,
        minutes: 90,
        pass: 70,
      },
    }
  }

  /// 限时（毫秒）。
  #[must_use]
  pub const fn duration_ms(self) -> i64 {
    self.minutes as i64 * 60 * 1000
  }
}

/// Fisher–Yates 洗牌，`rng` 返回 `[0, 1)` 区间的随机数。
pub fn shuffle_in_place<T>(items: &mut [T], rng: &mut impl FnMut() -> f64) {
  for i in (1..items.len()).rev() {
    let j = ((rng() * (i + 1) as f64).floor() as usize).min(i);
    items.swap(i, j);
  }
}

/// 返回洗牌后的副本。
#[must_use]
pub fn shuffled<T: Clone>(items: &[T], rng: &mut impl FnMut() -> f64) -> Vec<T> {
  let mut v = items.to_vec();
  shuffle_in_place(&mut v, rng);
  v
}

/// 按真实规则抽题：先按单选/多选配额随机抽取，不足时从剩余题目补齐，最后整体打乱。
///
/// 返回的是 `all` 中的下标。
#[must_use]
pub fn pick_exam(
  all: &[QuestionItem],
  rule: ExamRule,
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let (singles, multiples): (Vec<usize>, Vec<usize>) =
    (0..all.len()).partition(|&i| !all[i].is_multiple());
  let take_s = rule.singles.min(singles.len());
  let take_m = rule.multiples.min(multiples.len());
  let mut picked: Vec<usize> = shuffled(&singles, rng).into_iter().take(take_s).collect();
  picked.extend(shuffled(&multiples, rng).into_iter().take(take_m));
  if picked.len() < rule.total {
    let remaining: Vec<usize> = (0..all.len()).filter(|i| !picked.contains(i)).collect();
    let need = rule.total - picked.len();
    picked.extend(shuffled(&remaining, rng).into_iter().take(need));
  }
  let mut picked = shuffled(&picked, rng);
  picked.truncate(rule.total);
  picked
}

/// 成绩。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExamScore {
  /// 答对题数。
  pub correct: usize,
  /// 总题数。
  pub total: usize,
}

impl ExamScore {
  /// 计算成绩：`answer_of(question, pos)` 返回该题作答。
  #[must_use]
  pub fn calculate<'a, F>(questions: &'a [QuestionItem], mut answer_of: F) -> Self
  where
    F: FnMut(&'a QuestionItem, usize) -> Option<&'a [String]>,
  {
    let correct = questions
      .iter()
      .enumerate()
      .filter(|&(i, q)| q.is_answer_correct(answer_of(q, i).unwrap_or(&[])))
      .count();
    Self {
      correct,
      total: questions.len(),
    }
  }

  /// 是否合格。
  #[must_use]
  pub const fn is_passed(self, pass_line: usize) -> bool {
    self.correct >= pass_line
  }

  /// 正确率（四舍五入百分比）。
  #[must_use]
  pub fn percent(self) -> i64 {
    if self.total == 0 {
      return 0;
    }
    (self.correct as f64 / self.total as f64 * 100.0).round() as i64
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionType};

  fn q(id: usize, kind: QuestionType) -> QuestionItem {
    QuestionItem {
      id: Some(format!("A-{id}")),
      codes: Codes::default(),
      question: format!("q{id}"),
      options: vec![],
      answer_keys: vec!["A".into()],
      kind,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  fn lcg() -> impl FnMut() -> f64 {
    let mut s: u64 = 42;
    move || {
      s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
      (s >> 11) as f64 / (1u64 << 53) as f64
    }
  }

  #[test]
  fn picks_by_quota() {
    let mut all: Vec<_> = (0..100).map(|i| q(i, QuestionType::Single)).collect();
    all.extend((100..130).map(|i| q(i, QuestionType::Multiple)));
    let picked = pick_exam(&all, ExamRule::of(Bank::A), &mut lcg());
    assert_eq!(picked.len(), 40);
    assert_eq!(picked.iter().filter(|&&i| all[i].is_multiple()).count(), 8);
    let mut dedup = picked.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(dedup.len(), 40);
  }

  #[test]
  fn fills_up_when_multiples_are_short() {
    let mut all: Vec<_> = (0..50).map(|i| q(i, QuestionType::Single)).collect();
    all.push(q(50, QuestionType::Multiple));
    let picked = pick_exam(&all, ExamRule::of(Bank::A), &mut lcg());
    assert_eq!(picked.len(), 40);
  }

  #[test]
  fn scores_answers() {
    let qs = vec![q(1, QuestionType::Single), q(2, QuestionType::Single)];
    let right = vec!["A".to_owned()];
    let score = ExamScore::calculate(&qs, |_, i| (i == 0).then_some(right.as_slice()));
    assert_eq!(
      score,
      ExamScore {
        correct: 1,
        total: 2
      }
    );
    assert_eq!(score.percent(), 50);
    assert!(!score.is_passed(2));
  }
}
