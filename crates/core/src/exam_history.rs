//! 模拟考试成绩历史与备考状态评估。

use serde::{Deserialize, Serialize};

use crate::bank::Bank;
use crate::exam::ExamRule;

/// 评估时看最近几次考试。
pub const RECENT: usize = 5;
/// 至少考几次才给出判断。
pub const MIN_EXAMS: usize = 3;
/// 「可以去考了」要求最近每次都比合格线至少多答对的题数。
pub const SAFE_MARGIN: usize = 2;

/// 一条考试成绩记录（`localStorage` 中 `exam-history` 的元素）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExamRecord {
  /// 题库字母。
  pub bank: String,
  pub correct: usize,
  pub total: usize,
  /// 交卷时间（毫秒时间戳）。
  pub timestamp: i64,
  /// 是否为薄弱项组卷（题目偏难，不计入备考状态评估）。
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub weak: bool,
}

impl ExamRecord {
  /// 正确率（0–100）。
  #[must_use]
  pub fn percent(&self) -> f64 {
    if self.total == 0 {
      0.0
    } else {
      self.correct as f64 / self.total as f64 * 100.0
    }
  }

  /// 所属题库。
  #[must_use]
  pub fn bank(&self) -> Option<Bank> {
    self.bank.parse().ok()
  }
}

/// 备考状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
  /// 考试次数不足 [`MIN_EXAMS`]。
  NeedMore,
  /// 最近几次都稳定高于合格线。
  Ready,
  /// 多数能及格，但还不稳定。
  Almost,
  /// 多数不及格。
  NotYet,
}

impl Verdict {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::NeedMore => "再多考几次",
      Self::Ready => "可以去考了",
      Self::Almost => "接近了",
      Self::NotYet => "还需努力",
    }
  }
}

/// 某题库最近考试的评估结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Readiness {
  pub verdict: Verdict,
  /// 参与评估的考试次数（≤ [`RECENT`]）。
  pub count: usize,
  /// 其中及格次数。
  pub passed: usize,
  /// 平均正确率（0–100）。
  pub average: f64,
  /// 最近一次减去最早一次的正确率（百分点），用于显示进步 / 退步。
  pub trend: f64,
}

/// 某题库的全部常规模拟考试记录（时间升序，不含薄弱项组卷）。
#[must_use]
pub fn of_bank(history: &[ExamRecord], bank: Bank) -> Vec<&ExamRecord> {
  let mut v: Vec<&ExamRecord> = history
    .iter()
    .filter(|r| !r.weak && r.bank() == Some(bank))
    .collect();
  v.sort_by_key(|r| r.timestamp);
  v
}

/// 根据最近 [`RECENT`] 次考试判断备考状态；没有记录时返回 `None`。
#[must_use]
pub fn assess(history: &[ExamRecord], bank: Bank) -> Option<Readiness> {
  let all = of_bank(history, bank);
  let recent = &all[all.len().saturating_sub(RECENT)..];
  let (first, last) = (recent.first()?, recent.last()?);
  let rule = ExamRule::of(bank);
  let count = recent.len();
  let passed = recent.iter().filter(|r| r.correct >= rule.pass).count();
  let average = recent.iter().map(|r| r.percent()).sum::<f64>() / count as f64;
  let verdict = if count < MIN_EXAMS {
    Verdict::NeedMore
  } else if recent.iter().all(|r| r.correct >= rule.pass + SAFE_MARGIN) {
    Verdict::Ready
  } else if passed * 5 >= count * 3 {
    Verdict::Almost
  } else {
    Verdict::NotYet
  };
  Some(Readiness {
    verdict,
    count,
    passed,
    average,
    trend: last.percent() - first.percent(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn rec(bank: &str, correct: usize, ts: i64) -> ExamRecord {
    let total = ExamRule::of(bank.parse().expect("bank")).total;
    ExamRecord {
      bank: bank.into(),
      correct,
      total,
      timestamp: ts,
      weak: false,
    }
  }

  #[test]
  fn weak_exams_are_excluded_from_assessment() {
    let mut weak = rec("A", 0, 99);
    weak.weak = true;
    let h = vec![rec("A", 35, 1), weak.clone()];
    assert_eq!(of_bank(&h, Bank::A).len(), 1);
    let json = serde_json::to_string(&h[0]).expect("serialize");
    assert!(!json.contains("weak"), "常规记录不写 weak 字段，保持旧格式");
    let back: ExamRecord =
      serde_json::from_str(&serde_json::to_string(&weak).expect("serialize")).expect("parse");
    assert!(back.weak);
  }

  #[test]
  fn none_without_history() {
    assert_eq!(assess(&[rec("B", 50, 0)], Bank::A), None);
  }

  #[test]
  fn needs_minimum_exams() {
    let h = [rec("A", 40, 0), rec("A", 40, 1)];
    let r = assess(&h, Bank::A).expect("has records");
    assert_eq!((r.verdict, r.count, r.passed), (Verdict::NeedMore, 2, 2));
  }

  #[test]
  fn verdicts_use_recent_window_and_margin() {
    // A 类合格线 30 / 40。
    let mut h: Vec<ExamRecord> = (0..5).map(|i| rec("A", 10, i)).collect();
    h.extend((5..10).map(|i| rec("A", 33, i)));
    let r = assess(&h, Bank::A).expect("has records");
    assert_eq!(r.verdict, Verdict::Ready);
    assert_eq!(r.count, RECENT);

    let h = [rec("A", 30, 0), rec("A", 31, 1), rec("A", 35, 2)];
    assert_eq!(
      assess(&h, Bank::A).expect("records").verdict,
      Verdict::Almost
    );

    let h = [rec("A", 20, 0), rec("A", 25, 1), rec("A", 31, 2)];
    let r = assess(&h, Bank::A).expect("records");
    assert_eq!(r.verdict, Verdict::NotYet);
    assert!((r.trend - 27.5).abs() < 1e-9);
  }

  #[test]
  fn sorts_by_time_and_parses_legacy_json() {
    let json = r#"[{"bank":"A","correct":32,"total":40,"timestamp":2},{"bank":"A","correct":20,"total":40,"timestamp":1}]"#;
    let h: Vec<ExamRecord> = serde_json::from_str(json).expect("legacy format");
    let v = of_bank(&h, Bank::A);
    assert_eq!(v[0].timestamp, 1);
    assert!((assess(&h, Bank::A).expect("records").trend - 30.0).abs() < 1e-9);
  }
}
