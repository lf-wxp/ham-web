use ham_web_core::Bank;
use ham_web_core::ExamRule;
use ham_web_core::QuestionItem;
use ham_web_core::exam::pick_exam;
use ham_web_core::weak_exam;

use crate::util::random;

/// 抽一套试卷：常规按真实规则随机抽题；薄弱项组卷按分类正确率与错题加权。
pub(super) fn pick_paper(all: &[QuestionItem], bank: Bank, weak: bool) -> Vec<QuestionItem> {
  let rule = ExamRule::of(bank);
  let mut rng = random;
  let picked = if weak {
    let stats = crate::study::load_stats();
    let weights = weak_exam::weights(all, stats.bank(bank), &crate::study::load_book());
    weak_exam::pick(all, rule, &weights, &mut rng)
  } else {
    pick_exam(all, rule, &mut rng)
  };
  picked.into_iter().map(|i| all[i].clone()).collect()
}
