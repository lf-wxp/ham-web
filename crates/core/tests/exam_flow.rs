//! 集成测试：以外部使用者视角，走一遍「规则 → 抽题 → 计分」的完整考试流程。

use ham_web_core::exam::pick_exam;
use ham_web_core::{Bank, Codes, ExamRule, ExamScore, QuestionItem, QuestionOption, QuestionType};

fn question(id: &str, kind: QuestionType, answer: &str) -> QuestionItem {
  QuestionItem {
    id: Some(id.to_owned()),
    codes: Codes::default(),
    question: format!("题干 {id}"),
    options: vec![
      QuestionOption {
        key: "A".into(),
        text: "甲".into(),
      },
      QuestionOption {
        key: "B".into(),
        text: "乙".into(),
      },
      QuestionOption {
        key: "C".into(),
        text: "丙".into(),
      },
    ],
    answer_keys: answer.chars().map(|c| c.to_string()).collect(),
    kind,
    pages: None,
    image_url: None,
    explanation: None,
  }
}

/// 确定性随机源（LCG），保证抽题结果可复现。
fn lcg() -> impl FnMut() -> f64 {
  let mut s: u64 = 42;
  move || {
    s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    (s >> 11) as f64 / (1u64 << 53) as f64
  }
}

#[test]
fn exam_rules_are_self_consistent() {
  for bank in Bank::ALL {
    let rule = ExamRule::of(bank);
    assert_eq!(
      rule.singles + rule.multiples,
      rule.total,
      "{bank}: 单选+多选应等于总题数"
    );
    assert!(rule.pass <= rule.total, "{bank}: 合格线不应超过总题数");
  }
}

#[test]
fn picks_a_full_exam_with_correct_quotas() {
  let mut bank: Vec<QuestionItem> = (0..32)
    .map(|i| question(&format!("S{i}"), QuestionType::Single, "A"))
    .collect();
  bank.extend((0..8).map(|i| question(&format!("M{i}"), QuestionType::Multiple, "AB")));

  let rule = ExamRule::of(Bank::A);
  let picked = pick_exam(&bank, rule, &mut lcg());

  assert_eq!(picked.len(), rule.total);
  let singles = picked.iter().filter(|&&i| !bank[i].is_multiple()).count();
  let multiples = picked.iter().filter(|&&i| bank[i].is_multiple()).count();
  assert_eq!(singles, rule.singles);
  assert_eq!(multiples, rule.multiples);

  // 无重复抽题
  let mut dedup = picked.clone();
  dedup.sort_unstable();
  dedup.dedup();
  assert_eq!(dedup.len(), picked.len());
}

#[test]
fn scores_and_pass_line_work_end_to_end() {
  let qs = vec![
    question("1", QuestionType::Single, "A"),
    question("2", QuestionType::Single, "B"),
  ];

  // 全对
  let full = ExamScore::calculate(&qs, |q, _| Some(q.answer_keys.as_slice()));
  assert_eq!(full.correct, 2);
  assert!(full.is_passed(2));
  assert_eq!(full.percent(), 100);

  // 全错
  let none = ExamScore::calculate(&qs, |_, _| None::<&[String]>);
  assert_eq!(none.correct, 0);
  assert!(!none.is_passed(2));
  assert_eq!(none.percent(), 0);
}
