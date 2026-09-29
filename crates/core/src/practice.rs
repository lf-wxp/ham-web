//! 练习模式：题号/关键词搜索、跳转、跨题库「只看本类新增」过滤。

use std::collections::HashSet;

use crate::bank::Bank;
use crate::fingerprint::fingerprint;
use crate::question::QuestionItem;
use crate::text::js_trim;

/// 练习题序。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PracticeOrder {
  /// 顺序练习。
  #[default]
  Sequential,
  /// 随机练习。
  Random,
}

impl PracticeOrder {
  /// 存储值。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Sequential => "sequential",
      Self::Random => "random",
    }
  }

  /// 解析存储值。
  #[must_use]
  pub fn parse(s: &str) -> Option<Self> {
    match s {
      "sequential" => Some(Self::Sequential),
      "random" => Some(Self::Random),
      _ => None,
    }
  }

  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Sequential => "顺序练习",
      Self::Random => "随机练习",
    }
  }
}

/// 搜索结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
  /// 题目位置（0 起）。
  pub pos: usize,
  /// 首个题号，没有则为 `-`。
  pub j: String,
  /// 题干。
  pub text: String,
}

/// 搜索结果上限（与原实现一致：超过 50 条即停止）。
pub const SEARCH_LIMIT: usize = 50;

fn j_parts(q: &QuestionItem) -> Vec<String> {
  q.j_code()
    .map(|j| {
      j.to_uppercase()
        .split(',')
        .map(|s| js_trim(s).to_owned())
        .collect()
    })
    .unwrap_or_default()
}

/// 按题号片段或题干关键词搜索，`query_upper` 需为已 trim 并转大写的查询串。
#[must_use]
pub fn search<'a>(
  questions: impl IntoIterator<Item = &'a QuestionItem>,
  query_upper: &str,
) -> Vec<SearchMatch> {
  let mut results = Vec::new();
  if query_upper.is_empty() {
    return results;
  }
  for (pos, item) in questions.into_iter().enumerate() {
    let parts = j_parts(item);
    let by_j = parts.iter().any(|p| p.contains(query_upper));
    let by_text = item.question.to_uppercase().contains(query_upper);
    if by_j || by_text {
      results.push(SearchMatch {
        pos,
        j: parts
          .first()
          .filter(|s| !s.is_empty())
          .cloned()
          .unwrap_or_else(|| "-".to_owned()),
        text: item.question.clone(),
      });
    }
    if results.len() > SEARCH_LIMIT {
      break;
    }
  }
  results
}

/// 跳转定位：题号精确匹配 → 题号包含 → 题干包含。
#[must_use]
pub fn find_jump_target(questions: &[&QuestionItem], input: &str) -> Option<usize> {
  let raw = js_trim(input).to_uppercase();
  if raw.is_empty() {
    return None;
  }
  questions
    .iter()
    .position(|q| {
      let j = q.j_code().unwrap_or_default().to_uppercase();
      !j.is_empty() && (j == raw || j.split(',').any(|s| js_trim(s) == raw))
    })
    .or_else(|| {
      questions
        .iter()
        .position(|q| q.j_code().unwrap_or_default().to_uppercase().contains(&raw))
    })
    .or_else(|| {
      questions
        .iter()
        .position(|q| q.question.to_uppercase().contains(&raw))
    })
}

/// 「只看本类新增」：A 为基础全部保留；B 只保留 A 没有的；C 只保留 A、B 都没有的。
#[must_use]
pub fn unique_to_bank(
  bank: Bank,
  a: &[QuestionItem],
  b: &[QuestionItem],
  c: &[QuestionItem],
) -> Vec<QuestionItem> {
  let set = |qs: &[QuestionItem]| qs.iter().map(fingerprint).collect::<HashSet<_>>();
  match bank {
    Bank::A => a.to_vec(),
    Bank::B => {
      let fa = set(a);
      b.iter()
        .filter(|q| !fa.contains(&fingerprint(q)))
        .cloned()
        .collect()
    }
    Bank::C => {
      let (fa, fb) = (set(a), set(b));
      c.iter()
        .filter(|q| {
          let f = fingerprint(q);
          !fa.contains(&f) && !fb.contains(&f)
        })
        .cloned()
        .collect()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionType};

  fn q(j: &str, text: &str) -> QuestionItem {
    QuestionItem {
      id: None,
      codes: Codes {
        j: Some(j.into()),
        p: None,
      },
      question: text.into(),
      options: vec![],
      answer_keys: vec![],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    }
  }

  #[test]
  fn searches_and_jumps() {
    let qs = vec![q("LK0501", "天线增益"), q("LY0001,LY0002", "法规")];
    let m = search(&qs, "LY0002");
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].j, "LY0001");
    let refs: Vec<&QuestionItem> = qs.iter().collect();
    assert_eq!(find_jump_target(&refs, " ly0002 "), Some(1));
    assert_eq!(find_jump_target(&refs, "天线"), Some(0));
    assert_eq!(find_jump_target(&refs, "不存在"), None);
  }
}
