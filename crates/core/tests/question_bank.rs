//! 集成测试：以外部使用者视角，通过 `ham_web_core` 的公开 API 校验真实题库数据。
//!
//! 与 `src/` 下的单元测试不同，这里只能访问 crate 的公共接口，验证公开 API
//! 与 `public/questions/` 下真实 JSON 数据的一致性。

use std::collections::HashSet;

use ham_web_core::categories::top_of;
use ham_web_core::{Bank, BankConfig, ExamRule, QuestionItem, fingerprint};

fn bank_config() -> BankConfig {
  serde_json::from_str(include_str!("../../../public/questions/config.json"))
    .expect("config.json 应为合法 JSON")
}

fn bank_questions(bank: Bank) -> Vec<QuestionItem> {
  let json = match bank {
    Bank::A => include_str!("../../../public/questions/A.json"),
    Bank::B => include_str!("../../../public/questions/B.json"),
    Bank::C => include_str!("../../../public/questions/C.json"),
  };
  serde_json::from_str(json).expect("题库 JSON 应为合法 QuestionItem 数组")
}

#[test]
fn config_has_latest_version_and_resolves_urls() {
  let cfg = bank_config();
  assert!(!cfg.versions.is_empty(), "版本列表不应为空");
  let latest = cfg
    .sorted_versions()
    .first()
    .cloned()
    .expect("应有最新版本");
  assert!(latest.is_latest, "排序后的首个版本应标记为 isLatest");
  assert_eq!(latest.resolve_url(Bank::A), "/questions/A.json");
  assert_eq!(latest.resolve_url(Bank::B), "/questions/B.json");
  assert_eq!(latest.resolve_url(Bank::C), "/questions/C.json");
}

#[test]
fn every_bank_is_well_formed() {
  for bank in Bank::ALL {
    let qs = bank_questions(bank);
    assert!(!qs.is_empty(), "题库 {bank} 不应为空");

    let mut seen = HashSet::new();
    for q in &qs {
      // 题干与选项非空
      assert!(!q.question.trim().is_empty(), "题库 {bank} 存在空题干");
      assert!(!q.options.is_empty(), "题目 {:?} 缺少选项", q.id);

      // 答案非空，且是选项 key 的子集
      assert!(!q.answer_keys.is_empty(), "题目 {:?} 缺少答案", q.id);
      let option_keys: HashSet<&str> = q.options.iter().map(|o| o.key.as_str()).collect();
      assert!(
        q.answer_keys
          .iter()
          .all(|k| option_keys.contains(k.as_str())),
        "题目 {:?} 的答案不在选项内",
        q.id
      );

      // ID 唯一且前缀归属正确
      if let Some(id) = q.id_str() {
        assert!(seen.insert(id.to_owned()), "题库 {bank} 存在重复 ID {id}");
        assert_eq!(Bank::of_id(Some(id)), bank, "题目 {id} 归属的题库错误");
      }

      // 分类码能映射到已知一级分类
      if let Some(p) = q.p_code() {
        assert!(top_of(p).is_some(), "题目 {:?} 的分类码 {p} 未知", q.id);
      }
    }
  }
}

#[test]
fn each_bank_has_enough_questions_for_its_exam() {
  for bank in Bank::ALL {
    let qs = bank_questions(bank);
    let rule = ExamRule::of(bank);
    let singles = qs.iter().filter(|q| !q.is_multiple()).count();
    let multiples = qs.iter().filter(|q| q.is_multiple()).count();
    assert!(
      singles >= rule.singles,
      "{bank}: 单选题 {singles} 不足 {}",
      rule.singles
    );
    assert!(
      multiples >= rule.multiples,
      "{bank}: 多选题 {multiples} 不足 {}",
      rule.multiples
    );
    assert!(
      qs.len() >= rule.total,
      "{bank}: 总题数 {} 不足 {}",
      qs.len(),
      rule.total
    );
  }
}

#[test]
fn fingerprint_is_stable_for_known_question() {
  // 与 `src/fingerprint.rs` 单元测试中的历史 JS 输出保持一致，确保数据与算法逐字节吻合。
  let a = bank_questions(Bank::A);
  let first = a
    .iter()
    .find(|q| q.id_str() == Some("A-1"))
    .expect("题库 A 应包含 A-1");
  assert_eq!(
    fingerprint(first),
    "我国专门针对无线电管理的行政法规及其制定机构是：||A:《中华人民共和国无线电管理条例》|B:《中华人民共和国无线电管理办法》|C:国务院和中央军委|D:工业和信息化部||AC"
  );
}
