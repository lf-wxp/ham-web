//! 考试规则、抽题与计分。

use std::collections::BTreeMap;

use crate::bank::Bank;
use crate::categories::top_of;
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

/// 无分类码题目归入的兜底分类。
const OTHER: &str = "其他";

/// 在一组题目内，按一级分类占比分配 `quota` 个名额（贴近真实考试的知识点分布）。
fn pick_group_by_category(
  group: &[usize],
  quota: usize,
  all: &[QuestionItem],
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let mut by_cat: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();
  for &i in group {
    let cat = all[i].p_code().and_then(top_of).map_or(OTHER, |c| c.key);
    by_cat.entry(cat).or_default().push(i);
  }

  let total = group.len().max(1);
  let mut picked: Vec<usize> = Vec::new();
  for idxs in by_cat.values() {
    let share = (idxs.len() as f64 / total as f64 * quota as f64).round() as usize;
    picked.extend(shuffled(idxs, rng).into_iter().take(share));
  }
  if picked.len() < quota {
    let remaining: Vec<usize> = group
      .iter()
      .filter(|i| !picked.contains(i))
      .copied()
      .collect();
    picked.extend(
      shuffled(&remaining, rng)
        .into_iter()
        .take(quota - picked.len()),
    );
  }
  picked.truncate(quota);
  picked
}

/// 按真实规则抽题，并按一级分类占比分配名额（`pick_exam` 的「贴近真实」变体）。
///
/// 先按单选 / 多选配额拆分，每组内部再按分类占比精确分配，消除随机抽题的方差，
/// 使每次模拟考的知识点分布稳定、贴近官方大纲。返回的是 `all` 中的下标。
#[must_use]
pub fn pick_exam_by_category(
  all: &[QuestionItem],
  rule: ExamRule,
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let (singles, multiples): (Vec<usize>, Vec<usize>) =
    (0..all.len()).partition(|&i| !all[i].is_multiple());
  let take_s = rule.singles.min(singles.len());
  let take_m = rule.multiples.min(multiples.len());
  let mut picked = pick_group_by_category(&singles, take_s, all, rng);
  picked.extend(pick_group_by_category(&multiples, take_m, all, rng));
  if picked.len() < rule.total {
    let remaining: Vec<usize> = (0..all.len()).filter(|i| !picked.contains(i)).collect();
    picked.extend(
      shuffled(&remaining, rng)
        .into_iter()
        .take(rule.total - picked.len()),
    );
  }
  let mut picked = shuffled(&picked, rng);
  picked.truncate(rule.total);
  picked
}

/// 自定义组卷配置：自选题量、限时与一级分类范围。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomPaper {
  /// 单选题数。
  pub singles: usize,
  /// 多选题数。
  pub multiples: usize,
  /// 限定的一级分类 key（空 = 不限分类）。
  pub categories: Vec<&'static str>,
  /// 限时（分钟）；0 表示不限时。
  pub minutes: u32,
}

impl CustomPaper {
  /// 总题数。
  #[must_use]
  pub const fn total(&self) -> usize {
    self.singles + self.multiples
  }

  /// 生成对应的考试规则（合格线按 75% 取整，与 A/B/C 一致；`minutes == 0` 表示不限时）。
  #[must_use]
  pub fn rule(&self) -> ExamRule {
    ExamRule {
      total: self.total(),
      singles: self.singles,
      multiples: self.multiples,
      minutes: self.minutes,
      pass: ((self.total() as f64) * 0.75).round() as usize,
    }
  }
}

/// 按自定义配置抽题：先按一级分类过滤（可选），再按单选 / 多选配额抽取，最后整体打乱。
///
/// 分类或题型数量不足时从范围外补齐到目标题量（题型配额优先）。返回 `all` 中的下标。
#[must_use]
pub fn pick_custom(
  all: &[QuestionItem],
  paper: &CustomPaper,
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let total = paper.total();
  if total == 0 {
    return Vec::new();
  }
  let scope: Vec<usize> = if paper.categories.is_empty() {
    (0..all.len()).collect()
  } else {
    (0..all.len())
      .filter(|&i| {
        all[i]
          .p_code()
          .and_then(top_of)
          .is_some_and(|c| paper.categories.contains(&c.key))
      })
      .collect()
  };
  let (singles, multiples): (Vec<usize>, Vec<usize>) =
    scope.into_iter().partition(|&i| !all[i].is_multiple());
  let take_s = paper.singles.min(singles.len());
  let take_m = paper.multiples.min(multiples.len());
  let mut picked: Vec<usize> = shuffled(&singles, rng).into_iter().take(take_s).collect();
  picked.extend(shuffled(&multiples, rng).into_iter().take(take_m));
  if picked.len() < total {
    let remaining: Vec<usize> = (0..all.len()).filter(|i| !picked.contains(i)).collect();
    picked.extend(
      shuffled(&remaining, rng)
        .into_iter()
        .take(total - picked.len()),
    );
  }
  let mut picked = shuffled(&picked, rng);
  picked.truncate(total);
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
  fn picks_by_category_keeps_quota_and_distribution() {
    // 法规 80 题 + 天线 20 题（均单选），验证按分类占比分配 32 个单选名额。
    let mut all: Vec<_> = (0..80)
      .map(|i| {
        let mut item = q(i, QuestionType::Single);
        item.codes.p = Some("1.1.1".into()); // 法规
        item
      })
      .collect();
    all.extend((0..20).map(|i| {
      let mut item = q(100 + i, QuestionType::Single);
      item.codes.p = Some("3.3.1".into()); // 天线
      item
    }));
    // 8 个多选题（也归入法规），补齐总题数到 40。
    all.extend((0..8).map(|i| {
      let mut item = q(200 + i, QuestionType::Multiple);
      item.codes.p = Some("1.1.1".into());
      item
    }));
    let picked = pick_exam_by_category(&all, ExamRule::of(Bank::A), &mut lcg());
    assert_eq!(picked.len(), 40);
    // 单选里法规：天线 ≈ 4:1（32 个名额 → 约 26:6）。
    let law_singles = picked
      .iter()
      .filter(|&&i| !all[i].is_multiple() && all[i].codes.p.as_deref() == Some("1.1.1"))
      .count();
    assert!(
      (26..=32).contains(&law_singles),
      "law singles {law_singles}"
    );
    let mut dedup = picked.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(dedup.len(), 40);
  }

  #[test]
  fn picks_custom_by_category_and_quota() {
    // 法规 40 单选 + 天线 10 单选 + 5 多选（法规）。
    let mut all: Vec<_> = (0..40)
      .map(|i| {
        let mut item = q(i, QuestionType::Single);
        item.codes.p = Some("1.1.1".into());
        item
      })
      .collect();
    all.extend((0..10).map(|i| {
      let mut item = q(100 + i, QuestionType::Single);
      item.codes.p = Some("3.3.1".into());
      item
    }));
    all.extend((0..5).map(|i| {
      let mut item = q(200 + i, QuestionType::Multiple);
      item.codes.p = Some("1.1.1".into());
      item
    }));
    let paper = CustomPaper {
      singles: 6,
      multiples: 3,
      categories: vec!["法规"],
      minutes: 0,
    };
    let picked = pick_custom(&all, &paper, &mut lcg());
    assert_eq!(picked.len(), 9);
    // 仅来自「法规」分类。
    assert!(
      picked
        .iter()
        .all(|&i| all[i].codes.p.as_deref() == Some("1.1.1"))
    );
    // 单选 / 多选配额符合。
    assert_eq!(picked.iter().filter(|&&i| all[i].is_multiple()).count(), 3);
  }

  #[test]
  fn custom_paper_empty_returns_nothing() {
    let all: Vec<_> = (0..10).map(|i| q(i, QuestionType::Single)).collect();
    let picked = pick_custom(
      &all,
      &CustomPaper {
        singles: 0,
        multiples: 0,
        categories: vec![],
        minutes: 0,
      },
      &mut lcg(),
    );
    assert!(picked.is_empty());
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
