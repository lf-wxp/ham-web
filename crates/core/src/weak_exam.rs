//! 薄弱项组卷：按分类正确率、错题本与是否做过为每道题加权，加权不放回抽题；
//! 交卷后按一级分类对比本次与以往的正确率。
//!
//! 抽题权重优先取二级分类（官方分类码 P，即「知识点」）粒度的正确率，作答不足时
//! 回退到一级分类，再不足按未知正确率估计——把「薄弱大类」进一步定位到「薄弱知识点」。

use std::collections::{BTreeMap, HashSet};

use crate::categories::top_of;
use crate::exam::{ExamRule, shuffled};
use crate::mistake_book::{BankStats, MistakeBook, Tally, question_key};
use crate::question::QuestionItem;
use crate::question_stats::QuestionStats;

/// 错题本中的题额外乘的权重。
const MISTAKE_FACTOR: f64 = 3.0;
/// 从未做过的题额外乘的权重。
const UNSEEN_FACTOR: f64 = 1.5;
/// 一级分类作答少于这么多次时，视为正确率未知。
const MIN_CATEGORY_ANSWERED: u32 = 5;
/// 二级分类（知识点）作答少于这么多次时，视为正确率未知（知识点题量更少，门槛更低）。
const MIN_SUB_ANSWERED: u32 = 3;
/// 正确率未知的分类按这个正确率估计。
const UNKNOWN_RATE: f64 = 0.6;
/// 平均作答耗时超过这个毫秒数视为「会但慢」，额外加权（概念不牢的信号）。
const SLOW_MS: u64 = 45_000;
/// 「会但慢」的题额外乘的权重。
const SLOW_FACTOR: f64 = 1.5;

/// 某个分类码（P 码）的正确率：优先二级分类（知识点）粒度，作答不足时回退到
/// 一级分类，再不足返回 `None`。
fn category_rate(stats: Option<&BankStats>, code: &str) -> Option<f64> {
  let s = stats?;
  s.stats
    .subs
    .get(code)
    .filter(|t| t.answered >= MIN_SUB_ANSWERED)
    .and_then(|t| t.rate())
    .or_else(|| {
      top_of(code)
        .and_then(|top| s.stats.categories.get(top.key))
        .filter(|t| t.answered >= MIN_CATEGORY_ANSWERED)
        .and_then(|t| t.rate())
    })
}

/// 每道题的权重（与 `all` 一一对应）：`1 + 3 × 错误率`，错题 ×3，没做过 ×1.5，
/// 「会但慢」（平均作答耗时超过 [`SLOW_MS`]）再 ×[`SLOW_FACTOR`]。
#[must_use]
pub fn weights(
  all: &[QuestionItem],
  stats: Option<&BankStats>,
  book: &MistakeBook,
  qstats: &QuestionStats,
) -> Vec<f64> {
  let mistakes: HashSet<&str> = book.records.iter().map(|r| r.key.as_str()).collect();
  all
    .iter()
    .map(|q| {
      let rate = q
        .p_code()
        .and_then(|code| category_rate(stats, code))
        .unwrap_or(UNKNOWN_RATE);
      let key = question_key(q);
      let mut w = 1.0 + 3.0 * (1.0 - rate);
      if mistakes.contains(key.as_str()) {
        w *= MISTAKE_FACTOR;
      }
      if !stats.is_some_and(|s| s.seen.contains(&key)) {
        w *= UNSEEN_FACTOR;
      }
      if qstats.avg_ms(&key).is_some_and(|ms| ms > SLOW_MS) {
        w *= SLOW_FACTOR;
      }
      w
    })
    .collect()
}

/// 加权不放回抽取 `k` 个下标（Efraimidis–Spirakis：key = u^(1/w)，取最大的 k 个）。
fn weighted_take(
  candidates: &[usize],
  weights: &[f64],
  k: usize,
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let mut keyed: Vec<(f64, usize)> = candidates
    .iter()
    .map(|&i| {
      let w = weights.get(i).copied().unwrap_or(1.0).max(1e-6);
      (rng().max(f64::MIN_POSITIVE).powf(1.0 / w), i)
    })
    .collect();
  keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
  keyed.into_iter().take(k).map(|(_, i)| i).collect()
}

/// 与 [`crate::exam::pick_exam`] 相同的单选 / 多选配额，但按 `weights` 加权抽题。
#[must_use]
pub fn pick(
  all: &[QuestionItem],
  rule: ExamRule,
  weights: &[f64],
  rng: &mut impl FnMut() -> f64,
) -> Vec<usize> {
  let (singles, multiples): (Vec<usize>, Vec<usize>) =
    (0..all.len()).partition(|&i| !all[i].is_multiple());
  let mut picked = weighted_take(&singles, weights, rule.singles, rng);
  picked.extend(weighted_take(&multiples, weights, rule.multiples, rng));
  if picked.len() < rule.total {
    let taken: HashSet<usize> = picked.iter().copied().collect();
    let remaining: Vec<usize> = (0..all.len()).filter(|i| !taken.contains(i)).collect();
    picked.extend(weighted_take(
      &remaining,
      weights,
      rule.total - picked.len(),
      rng,
    ));
  }
  let mut picked = shuffled(&picked, rng);
  picked.truncate(rule.total);
  picked
}

/// 某一级分类本次与以往的对比。
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryDelta {
  pub key: &'static str,
  pub name: &'static str,
  /// 本次考试中该分类的作答情况。
  pub exam: Tally,
  /// 交卷前该分类的累计正确率。
  pub before: Option<f64>,
}

impl CategoryDelta {
  /// 本次正确率减以往正确率（百分点）。
  #[must_use]
  pub fn change(&self) -> Option<f64> {
    Some((self.exam.rate()? - self.before?) * 100.0)
  }
}

/// 按一级分类汇总本次考试（`correct_of(pos)` 返回第 pos 题是否答对），与交卷前的统计对比；
/// 按本次题数从多到少排列。
#[must_use]
pub fn compare(
  questions: &[QuestionItem],
  mut correct_of: impl FnMut(usize) -> bool,
  before: Option<&BankStats>,
) -> Vec<CategoryDelta> {
  let mut map: BTreeMap<&'static str, (&'static str, Tally)> = BTreeMap::new();
  for (pos, q) in questions.iter().enumerate() {
    let Some(top) = q.p_code().and_then(top_of) else {
      continue;
    };
    let entry = map.entry(top.key).or_insert((top.name, Tally::default()));
    entry.1.answered += 1;
    entry.1.correct += u32::from(correct_of(pos));
  }
  let mut out: Vec<CategoryDelta> = map
    .into_iter()
    .map(|(key, (name, exam))| CategoryDelta {
      key,
      name,
      exam,
      before: before
        .and_then(|s| s.stats.categories.get(key))
        .and_then(|t| t.rate()),
    })
    .collect();
  out.sort_by_key(|d| std::cmp::Reverse(d.exam.answered));
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bank::Bank;
  use crate::question::{Codes, QuestionOption, QuestionType};
  use crate::question_stats::QuestionStats;

  fn q(n: usize, p: &str, multiple: bool) -> QuestionItem {
    QuestionItem {
      id: Some(format!("A-{n}")),
      codes: Codes {
        j: Some(format!("LY{n:04}")),
        p: Some(p.into()),
      },
      question: format!("题目 {n}"),
      options: vec![
        QuestionOption {
          key: "A".into(),
          text: "x".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "y".into(),
        },
      ],
      answer_keys: if multiple {
        vec!["A".into(), "B".into()]
      } else {
        vec!["A".into()]
      },
      kind: if multiple {
        QuestionType::Multiple
      } else {
        QuestionType::Single
      },
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  fn lcg(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
      s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
      #[allow(clippy::cast_precision_loss)]
      let v = (s >> 11) as f64 / (1u64 << 53) as f64;
      v
    }
  }

  /// 两个不同一级分类的分类码。
  fn two_tops() -> (&'static str, &'static str) {
    let mut codes = crate::categories::SUB_CATEGORIES.iter();
    let first = codes.next().map_or("", |s| s.code);
    let top = top_of(first).map(|t| t.key);
    let second = codes
      .find(|s| top_of(s.code).map(|t| t.key) != top)
      .map_or("", |s| s.code);
    (first, second)
  }

  #[test]
  fn weak_categories_and_mistakes_weigh_more() {
    let (weak, strong) = two_tops();
    let all = vec![q(1, weak, false), q(2, strong, false), q(3, strong, false)];
    let mut stats = crate::mistake_book::StudyStats::default();
    for _ in 0..10 {
      stats.record(&all[0], false);
      stats.record(&all[1], true);
    }
    stats.record(&all[2], true);
    let mut book = MistakeBook::default();
    book.record(&all[2], &["B".to_owned()], 0);
    let w = weights(&all, stats.bank(Bank::A), &book, &QuestionStats::default());
    assert!((w[0] - 4.0).abs() < 1e-9, "全错的分类：1 + 3 × 1");
    assert!((w[1] - 1.0).abs() < 1e-9, "全对且做过");
    assert!((w[2] - 3.0).abs() < 1e-9, "全对分类但在错题本：1 × 3");
  }

  #[test]
  fn slow_but_correct_weighs_more() {
    let (a, _) = two_tops();
    let all = vec![q(1, a, false)];
    let mut stats = crate::mistake_book::StudyStats::default();
    for _ in 0..5 {
      stats.record(&all[0], true); // 已充分作答且全对
    }
    let mut qstats = QuestionStats::default();
    qstats.record(&question_key(&all[0]), true, 0, SLOW_MS + 1);
    let w = weights(&all, stats.bank(Bank::A), &MistakeBook::default(), &qstats);
    // 基础 1（全对且做过），会但慢 ×1.5。
    assert!((w[0] - 1.5).abs() < 1e-9, "会但慢应加权 1.5，实际 {}", w[0]);
  }

  #[test]
  fn weak_sub_category_weighs_more_than_strong_within_same_top() {
    // 1.1.1 与 1.1.2 同属「法规」一级分类，但知识点不同：细分后应区分开。
    let weak = "1.1.1";
    let strong = "1.1.2";
    let all = vec![q(1, weak, false), q(2, strong, false)];
    let mut stats = crate::mistake_book::StudyStats::default();
    for _ in 0..5 {
      stats.record(&all[0], false);
      stats.record(&all[1], true);
    }
    let w = weights(
      &all,
      stats.bank(Bank::A),
      &MistakeBook::default(),
      &QuestionStats::default(),
    );
    // 同一一级分类下，薄弱知识点的题权重应更高（1 + 3×1 = 4 vs 1）。
    assert!(
      w[0] > w[1],
      "薄弱知识点应加权更高：weak={} strong={}",
      w[0],
      w[1]
    );
    assert!((w[0] - 4.0).abs() < 1e-9, "薄弱知识点：1 + 3 × 1");
    assert!((w[1] - 1.0).abs() < 1e-9, "已掌握知识点：1 + 3 × 0");
  }

  #[test]
  fn pick_respects_quota_and_prefers_heavy_questions() {
    let (a, b) = two_tops();
    let mut all: Vec<QuestionItem> = (0..40)
      .map(|n| q(n, if n < 20 { a } else { b }, false))
      .collect();
    all.extend((40..50).map(|n| q(n, a, true)));
    let rule = ExamRule {
      total: 10,
      singles: 8,
      multiples: 2,
      minutes: 10,
      pass: 6,
    };
    let mut weights = vec![1.0; all.len()];
    for w in &mut weights[..20] {
      *w = 50.0;
    }
    let mut rng = lcg(7);
    let mut heavy = 0;
    for _ in 0..50 {
      let picked = pick(&all, rule, &weights, &mut rng);
      assert_eq!(picked.len(), 10);
      assert_eq!(picked.iter().filter(|&&i| all[i].is_multiple()).count(), 2);
      let unique: HashSet<usize> = picked.iter().copied().collect();
      assert_eq!(unique.len(), 10);
      heavy += picked.iter().filter(|&&i| i < 20).count();
    }
    assert!(heavy > 50 * 7, "加权后绝大多数单选来自高权重题：{heavy}");
  }

  #[test]
  fn compare_groups_by_top_category() {
    let (a, b) = two_tops();
    let qs = vec![q(1, a, false), q(2, a, false), q(3, b, false)];
    let mut stats = crate::mistake_book::StudyStats::default();
    stats.record(&qs[0], false);
    stats.record(&qs[0], true);
    let rows = compare(&qs, |pos| pos != 1, stats.bank(Bank::A));
    assert_eq!(rows.len(), 2);
    assert_eq!(
      rows[0].exam,
      Tally {
        answered: 2,
        correct: 1
      }
    );
    assert_eq!(rows[0].before, Some(0.5));
    assert_eq!(rows[0].change(), Some(0.0));
    assert_eq!(rows[1].before, None);
  }
}
