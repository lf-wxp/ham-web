//! 题目内容指纹。
//!
//! 指纹 = 规整后的题干 + 选项（`key:text`，`|` 连接）+ 排序后的答案，
//! 用 `||` 分隔。它与题号、题目顺序、所属题库都无关：
//!
//! - 解析文件 `data/explanations.json` 以指纹为 key，题库重排/增删不会错位；
//! - A/B/C 三个题库中内容相同的题目指纹相同，用于识别跨题库重合题（「只看本类新增」）。
//!
//! 算法必须与历史数据保持逐字节一致，请勿随意修改。

use crate::question::{QuestionItem, QuestionOption};
use crate::text::{collapse_whitespace, is_js_whitespace};

/// 由原始字段计算指纹。
#[must_use]
pub fn fingerprint_parts(
  question: &str,
  options: &[QuestionOption],
  answer_keys: &[String],
) -> String {
  let stem = collapse_whitespace(question);
  let opts = options
    .iter()
    .map(|o| format!("{}:{}", o.key, collapse_whitespace(&o.text)))
    .collect::<Vec<_>>()
    .join("|");
  let mut ans: Vec<&str> = answer_keys.iter().map(String::as_str).collect();
  ans.sort_unstable();
  format!("{stem}||{opts}||{}", ans.concat())
}

/// 计算题目的内容指纹。
#[must_use]
pub fn fingerprint(q: &QuestionItem) -> String {
  fingerprint_parts(&q.question, &q.options, &q.answer_keys)
}

/// 忽略全部空白的内容 key（题干 + 选项 + 答案）。
///
/// 指纹保留 `collapse_whitespace` 的单个空格，因此同一道题在题库之间由 CSV 带来的
/// 空格差异（「25 瓦」与「25瓦」）会产生多个指纹。跨题库判断「是不是同一道题」时
/// 用本函数去掉全部空白，避免同一道题被当成两道题（`unique_to_bank` 的「只看本类新增」、
/// 解析表的多指纹同步都依赖它）。
///
/// 与 [`fingerprint`] 的用途区分：`fingerprint` 是持久化 key（解析表 key、错题本 key 的
/// 哈希输入），必须保持逐字节稳定；`content_key` 只用于内存中的同题判定。
#[must_use]
pub fn content_key(q: &QuestionItem) -> String {
  let stem = strip_whitespace(&q.question);
  let opts = q
    .options
    .iter()
    .map(|o| format!("{}:{}", o.key, strip_whitespace(&o.text)))
    .collect::<Vec<_>>()
    .join("|");
  let mut ans: Vec<&str> = q.answer_keys.iter().map(String::as_str).collect();
  ans.sort_unstable();
  format!("{stem}||{opts}||{}", ans.concat())
}

/// 去掉全部 JS 空白字符。
#[must_use]
pub fn strip_whitespace(s: &str) -> String {
  s.chars().filter(|c| !is_js_whitespace(*c)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_legacy_js_output() {
    let options = vec![
      QuestionOption {
        key: "A".into(),
        text: "《中华人民共和国无线电管理条例》".into(),
      },
      QuestionOption {
        key: "B".into(),
        text: "《中华人民共和国无线电管理办法》".into(),
      },
      QuestionOption {
        key: "C".into(),
        text: "国务院和中央军委".into(),
      },
      QuestionOption {
        key: "D".into(),
        text: "工业和信息化部".into(),
      },
    ];
    let fp = fingerprint_parts(
      "我国专门针对无线电管理的行政法规及其制定机构是：",
      &options,
      &["C".into(), "A".into()],
    );
    assert_eq!(
      fp,
      "我国专门针对无线电管理的行政法规及其制定机构是：||A:《中华人民共和国无线电管理条例》|B:《中华人民共和国无线电管理办法》|C:国务院和中央军委|D:工业和信息化部||AC"
    );
  }

  fn variant(question: &str, option: &str) -> QuestionItem {
    QuestionItem {
      id: Some("B-1".into()),
      codes: crate::question::Codes::default(),
      question: question.into(),
      options: vec![QuestionOption {
        key: "A".into(),
        text: option.into(),
      }],
      answer_keys: vec!["A".into()],
      kind: crate::question::QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  #[test]
  fn content_key_ignores_whitespace_variants() {
    let spaced = variant("执照的有效期不超过：", "自 2024 年 3 月 1 日起施行");
    let tight = variant("执照的有效期不超过：", "自2024年3月1日起施行");
    // 指纹保留单个空格，因此题库之间的空格差异会产生两个指纹
    assert_ne!(fingerprint(&spaced), fingerprint(&tight));
    // 内容 key 忽略全部空白，判重/同步都用它
    assert_eq!(content_key(&spaced), content_key(&tight));
    // 内容不同则必须区分
    let other = variant("执照的有效期不超过：", "自2025年3月1日起施行");
    assert_ne!(content_key(&spaced), content_key(&other));
  }
}
