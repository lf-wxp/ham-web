//! 题库摘要：记住上次见到的题库内容，题库更新后告诉用户新增 / 修改 / 删除了多少题、补了多少解析。
//!
//! 每题只存内容指纹的 32 位 FNV-1a 哈希（约 700 题 ≈ 7 KB），足以统计差异又不占太多本地存储。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::fingerprint::fingerprint;
use crate::question::QuestionItem;

fn fnv1a(s: &str) -> u32 {
  s.bytes().fold(0x811c_9dc5, |h, b| {
    (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
  })
}

fn has_explanation(q: &QuestionItem) -> bool {
  q.explanation
    .as_deref()
    .is_some_and(|e| !e.trim().is_empty())
}

/// 某个题库的内容摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
  /// 题目指纹哈希（升序）。
  pub hashes: Vec<u32>,
  /// 有解析的题目数。
  pub explained: usize,
}

impl Digest {
  #[must_use]
  pub fn of(questions: &[QuestionItem]) -> Self {
    let mut hashes: Vec<u32> = questions.iter().map(|q| fnv1a(&fingerprint(q))).collect();
    hashes.sort_unstable();
    Self {
      hashes,
      explained: questions.iter().filter(|q| has_explanation(q)).count(),
    }
  }
}

/// 两次题库内容的差异。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Diff {
  /// 新出现的题目（新增或内容被修改）。
  pub added: usize,
  /// 消失的题目（删除或内容被修改）。
  pub removed: usize,
  /// 有解析题目数的变化。
  pub explained: isize,
  pub total: usize,
}

impl Diff {
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.added == 0 && self.removed == 0 && self.explained == 0
  }

  /// 面向用户的一句话描述，如「新增或修改 12 题，删除 3 题，新增解析 40 条」。
  #[must_use]
  pub fn describe(&self) -> String {
    let mut parts = Vec::new();
    // 修改一道题表现为「一增一删」，合并成「修改」更好理解
    let changed = self.added.min(self.removed);
    let (added, removed) = (self.added - changed, self.removed - changed);
    if changed > 0 {
      parts.push(format!("修改 {changed} 题"));
    }
    if added > 0 {
      parts.push(format!("新增 {added} 题"));
    }
    if removed > 0 {
      parts.push(format!("删除 {removed} 题"));
    }
    match self.explained {
      n if n > 0 => parts.push(format!("新增解析 {n} 条")),
      n if n < 0 => parts.push(format!("解析减少 {} 条", n.unsigned_abs())),
      _ => {}
    }
    if parts.is_empty() {
      parts.push("内容有更新".to_owned());
    }
    format!("{}，现共 {} 题", parts.join("，"), self.total)
  }
}

/// 比较两个摘要（按多重集合计数，重复题目也能正确统计）。
#[must_use]
pub fn diff(old: &Digest, new: &Digest) -> Diff {
  let count = |hashes: &[u32], others: &[u32]| {
    let other: HashSet<u32> = others.iter().copied().collect();
    hashes.iter().filter(|h| !other.contains(h)).count()
  };
  #[allow(clippy::cast_possible_wrap)]
  let explained = new.explained as isize - old.explained as isize;
  Diff {
    added: count(&new.hashes, &old.hashes),
    removed: count(&old.hashes, &new.hashes),
    explained,
    total: new.hashes.len(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionOption, QuestionType};

  fn q(text: &str, explanation: Option<&str>) -> QuestionItem {
    QuestionItem {
      id: None,
      codes: Codes { j: None, p: None },
      question: text.into(),
      options: vec![
        QuestionOption {
          key: "A".into(),
          text: "是".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "否".into(),
        },
      ],
      answer_keys: vec!["A".into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: explanation.map(Into::into),
    }
  }

  #[test]
  fn reports_changes() {
    let old = Digest::of(&[q("一", None), q("二", None), q("三", None)]);
    let new = Digest::of(&[
      q("一", Some("解析")),
      q("二改", None),
      q("四", Some("x")),
      q("五", None),
    ]);
    let d = diff(&old, &new);
    assert_eq!((d.added, d.removed, d.explained, d.total), (3, 2, 2, 4));
    assert_eq!(
      d.describe(),
      "修改 2 题，新增 1 题，新增解析 2 条，现共 4 题"
    );
  }

  #[test]
  fn explanation_only_and_identical() {
    let old = Digest::of(&[q("一", None)]);
    let new = Digest::of(&[q("一", Some("解析"))]);
    assert_eq!(diff(&old, &new).describe(), "新增解析 1 条，现共 1 题");
    assert!(diff(&new, &new).is_empty());
    // 摘要可序列化存储
    let json = serde_json::to_string(&new).unwrap();
    assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), new);
  }
}
