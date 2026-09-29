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
use crate::text::collapse_whitespace;

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
}
