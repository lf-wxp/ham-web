//! 模拟考试的逐题复盘快照。
//!
//! 交卷结果原本只存在于当次弹窗里，关掉就再也回不去「哪道题错了、正确答案是什么」。
//! 这里把最近一次考试的作答存下来供复盘页使用。
//!
//! **只保留最近一次**：一次 C 类考试 90 题，逐题存作答与解析约几十 KB，若像成绩历史
//! 那样存 50 次会很快撑满 `localStorage`（约 5 MB），进而影响错题本等更重要的本地数据。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 单题复盘项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewItem {
  /// 题目稳定 ID（用于跳转收藏 / 错题本）。
  pub id: String,
  /// 官方题号（展示用，可能为空）。
  pub code: String,
  /// 题干。
  pub question: String,
  /// 选项文本，形如 `A. 是`。
  pub options: Vec<String>,
  /// 正确答案字母。
  pub answer: Vec<String>,
  /// 用户作答字母（空表示未答）。
  pub given: Vec<String>,
  /// 解析（可能为空）。
  pub explanation: String,
  /// 二级分类码 `P`（用于定位薄弱点）。
  pub category: String,
}

impl ReviewItem {
  /// 是否作答正确：与标准答案**集合一致**（顺序无关，与 [`crate::question::same_set`]
  /// 的计分口径一致 —— 多选题以任意顺序选全所有正确选项都算对）。
  #[must_use]
  pub fn is_correct(&self) -> bool {
    !self.given.is_empty() && crate::question::same_set(&self.given, &self.answer)
  }

  /// 是否未作答。
  #[must_use]
  pub fn is_blank(&self) -> bool {
    self.given.is_empty()
  }
}

/// 一次考试的完整复盘快照。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExamReview {
  /// 题库类别（`A` / `B` / `C`）。
  pub bank: String,
  /// 交卷时间（毫秒时间戳）。
  pub finished_at_ms: i64,
  /// 逐题项（按考试中的题序）。
  pub items: Vec<ReviewItem>,
}

/// 某个分类的复盘统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryReview {
  /// 二级分类码。
  pub code: String,
  /// 该分类题目数。
  pub total: usize,
  /// 答对数。
  pub correct: usize,
}

impl ExamReview {
  /// 已作答题数。
  #[must_use]
  pub fn answered(&self) -> usize {
    self.items.iter().filter(|i| !i.is_blank()).count()
  }

  /// 答对题数。
  #[must_use]
  pub fn correct_count(&self) -> usize {
    self.items.iter().filter(|i| i.is_correct()).count()
  }

  /// 总题数。
  #[must_use]
  pub fn total(&self) -> usize {
    self.items.len()
  }

  /// 答错的题（含未答），按题序。
  #[must_use]
  pub fn wrong_items(&self) -> Vec<&ReviewItem> {
    self.items.iter().filter(|i| !i.is_correct()).collect()
  }

  /// 按二级分类聚合，按错误数从多到少排序（错误数相同时题多的在前）。
  ///
  /// 用于回答「这次到底栽在哪一类」。
  #[must_use]
  pub fn by_category(&self) -> Vec<CategoryReview> {
    // 用 `HashMap` 聚合而不是对每个分类线性查找：一次考试可能有几十个二级分类，
    // 逐题线性扫描是 O(分类数 × 题数)。
    let mut map: HashMap<&str, (usize, usize)> = HashMap::new();
    for item in &self.items {
      let e = map.entry(item.category.as_str()).or_insert((0, 0));
      e.0 += 1;
      e.1 += usize::from(item.is_correct());
    }
    let mut map: Vec<CategoryReview> = map
      .into_iter()
      .map(|(code, (total, correct))| CategoryReview {
        code: code.to_owned(),
        total,
        correct,
      })
      .collect();
    map.sort_by(|a, b| {
      let wrong = |c: &CategoryReview| c.total - c.correct;
      wrong(b)
        .cmp(&wrong(a))
        .then(b.total.cmp(&a.total))
        // `HashMap` 的迭代顺序不保证稳定，同分时用分类码兜底，保证同样的数据
        // 每次得到同样的顺序（也让快照测试可重复）。
        .then_with(|| a.code.cmp(&b.code))
    });
    map
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn item(category: &str, answer: &[&str], given: &[&str]) -> ReviewItem {
    ReviewItem {
      id: format!("A-{category}"),
      code: "LK0001".to_owned(),
      question: "题干".to_owned(),
      options: vec!["A. 选项".to_owned()],
      answer: answer.iter().map(|s| (*s).to_owned()).collect(),
      given: given.iter().map(|s| (*s).to_owned()).collect(),
      explanation: String::new(),
      category: category.to_owned(),
    }
  }

  #[test]
  fn multiple_choice_must_match_exactly() {
    let q = item("1.1.1", &["A", "C"], &["A"]);
    // 多选只选了部分答案：按计分规则不得分。
    assert!(!q.is_correct());
    let full = item("1.1.1", &["A", "C"], &["A", "C"]);
    assert!(full.is_correct());
    // 顺序无关：以不同顺序选全所有正确选项同样算对（与 same_set 计分一致）。
    let reordered = item("1.1.1", &["A", "C"], &["C", "A"]);
    assert!(reordered.is_correct());
  }

  #[test]
  fn blank_is_not_correct() {
    let q = item("1.1.1", &["A"], &[]);
    assert!(q.is_blank());
    assert!(!q.is_correct());
  }

  #[test]
  fn counts_and_wrong_items() {
    let review = ExamReview {
      bank: "A".to_owned(),
      finished_at_ms: 0,
      items: vec![
        item("1.1.1", &["A"], &["A"]),
        item("1.1.1", &["B"], &["A"]),
        item("2.2.2", &["C"], &[]),
      ],
    };
    assert_eq!(review.total(), 3);
    assert_eq!(review.answered(), 2);
    assert_eq!(review.correct_count(), 1);
    assert_eq!(review.wrong_items().len(), 2);
  }

  #[test]
  fn category_ranking_puts_weakest_first() {
    let review = ExamReview {
      bank: "C".to_owned(),
      finished_at_ms: 0,
      items: vec![
        // 3.3.3 错了 2 题，是最弱项
        item("3.3.3", &["A"], &["B"]),
        item("3.3.3", &["A"], &["B"]),
        // 1.1.1 错了 1 题
        item("1.1.1", &["A"], &["B"]),
        item("1.1.1", &["A"], &["A"]),
      ],
    };
    let cats = review.by_category();
    assert_eq!(cats[0].code, "3.3.3");
    assert_eq!(cats[0].total, 2);
    assert_eq!(cats[0].correct, 0);
    assert_eq!(cats[1].code, "1.1.1");
    assert_eq!(cats[1].correct, 1);
  }
}
