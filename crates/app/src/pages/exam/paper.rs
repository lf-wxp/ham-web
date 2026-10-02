use ham_web_core::Bank;
use ham_web_core::ExamRule;
use ham_web_core::QuestionItem;
use ham_web_core::exam::{CustomPaper, pick_custom, pick_exam, pick_exam_by_category};
use ham_web_core::weak_exam;

use crate::util::random;

/// 抽一套试卷：常规按真实规则随机抽题（可切换「按分类占比」）；薄弱项组卷按分类正确率与
/// 错题加权；自定义组卷按用户配置的题量与分类范围抽题。
pub(super) fn pick_paper(
  all: &[QuestionItem],
  bank: Bank,
  weak: bool,
  weighted: bool,
  custom: Option<&CustomPaper>,
) -> Vec<QuestionItem> {
  let rule = ExamRule::of(bank);
  let mut rng = random;
  let picked = if let Some(paper) = custom {
    pick_custom(all, paper, &mut rng)
  } else if weak {
    let stats = crate::study::load_stats();
    let weights = weak_exam::weights(
      all,
      stats.bank(bank),
      &crate::study::load_book(),
      &crate::study::load_qstats(),
    );
    weak_exam::pick(all, rule, &weights, &mut rng)
  } else if weighted {
    pick_exam_by_category(all, rule, &mut rng)
  } else {
    pick_exam(all, rule, &mut rng)
  };
  picked.into_iter().map(|i| all[i].clone()).collect()
}
