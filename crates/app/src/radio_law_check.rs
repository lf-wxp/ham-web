//! 法规原文库「条文 → 题目」映射的一致性校验。
//!
//! 映射表在 `ham_web_core::radio_law::ARTICLE_QUESTIONS`（人工维护），题目数据在
//! `public/questions/*.json`（题库本身）—— 两处都读得到源码，就钉成一条测试：
//! 每个映射的题目 id 必须在 A 卷里真实存在，否则页面上的反向链接会指向不存在的题目。

#[cfg(test)]
mod tests {
  use ham_web_core::radio_law::ARTICLE_QUESTIONS;

  /// 题库文件在 app crate 之外，`include_str!` 以本文件为基准向上走。
  const BANK_A: &str = include_str!("../../../public/questions/A.json");

  #[test]
  fn every_mapped_question_exists_in_the_bank() {
    let bank: serde_json::Value = serde_json::from_str(BANK_A).expect("A 卷应当是合法 JSON");
    let questions: Vec<&serde_json::Value> = match &bank {
      serde_json::Value::Array(items) => items.iter().collect(),
      serde_json::Value::Object(map) => map
        .get("questions")
        .and_then(|q| q.as_array())
        .expect("题库应当有 questions 数组")
        .iter()
        .collect(),
      _ => panic!("题库结构变了，先看看 A.json"),
    };
    let by_id: std::collections::HashMap<&str, &serde_json::Value> = questions
      .iter()
      .filter_map(|q| q.get("id").and_then(|v| v.as_str()).map(|id| (id, *q)))
      .collect();

    for m in ARTICLE_QUESTIONS {
      for q in m.questions {
        let item = by_id
          .get(q.id)
          .unwrap_or_else(|| panic!("{} 第{}条映射的题目 {} 不在 A 卷里", m.law, m.number, q.id));
        // 题干逐字节一致：题目改了这里不跟，反向链接上挂的就是过期题干。
        let stem = item
          .get("question")
          .and_then(|v| v.as_str())
          .unwrap_or_default();
        assert_eq!(q.stem, stem, "{} 的题干与题库不一致", q.id);
        // 知识点一致：反向链接 `/practice?sub=<code>` 才指向对的地方。
        let p = item
          .get("codes")
          .and_then(|v| v.get("P"))
          .and_then(|v| v.as_str())
          .unwrap_or_default();
        assert_eq!(q.p_code, p, "{} 的知识点 code 与题库不一致", q.id);
      }
    }
    // 刻意交叉：A-119 / A-120（办法第三条与第四十三条的「商业」是一件事的两处规定）、
    // A-149（办法第四十七条的罚则就是指向条例第七十条 —— 一部规章、一部行政法规）。
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for q in ARTICLE_QUESTIONS.iter().flat_map(|m| m.questions.iter()) {
      *counts.entry(q.id).or_default() += 1;
    }
    for (id, n) in &counts {
      if *n > 1 {
        assert!(
          matches!(*id, "A-119" | "A-120" | "A-149"),
          "{id} 被映射了 {n} 次（刻意交叉只该是 A-119 / A-120 / A-149）"
        );
      }
    }
  }
}
