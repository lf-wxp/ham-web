//! 题目数据结构，与 `public/questions/*.json` 的 JSON 格式一一对应。

use serde::{Deserialize, Serialize};

/// 选项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
  /// 选项字母，如 `A`。
  #[serde(default)]
  pub key: String,
  /// 选项文本。
  #[serde(default)]
  pub text: String,
}

/// 官方编码：`J` 为题号（可能是逗号分隔的多个），`P` 为分类码（如 `1.1.1`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Codes {
  /// 题号。
  #[serde(rename = "J", default)]
  pub j: Option<String>,
  /// 分类码。
  #[serde(rename = "P", default)]
  pub p: Option<String>,
}

/// 教材页码范围（历史字段，当前数据集不再输出）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pages {
  /// 起始页。
  pub start: Option<u32>,
  /// 结束页。
  pub end: Option<u32>,
}

/// 题型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionType {
  /// 单选题。
  Single,
  /// 多选题。
  Multiple,
}

impl QuestionType {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Single => "单选",
      Self::Multiple => "多选",
    }
  }
}

/// 一道题目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionItem {
  /// 题库内唯一 ID，形如 `A-1`、`B-23`。
  #[serde(default)]
  pub id: Option<String>,
  /// 官方编码。
  #[serde(default)]
  pub codes: Codes,
  /// 题干。
  pub question: String,
  /// 选项列表。
  #[serde(default)]
  pub options: Vec<QuestionOption>,
  /// 正确答案字母列表。
  #[serde(default)]
  pub answer_keys: Vec<String>,
  /// 题型。
  #[serde(rename = "type")]
  pub kind: QuestionType,
  /// 页码（可选）。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub pages: Option<Pages>,
  /// 附图地址，如 `/questions/images/0.jpg`。
  #[serde(rename = "imageUrl", default)]
  pub image_url: Option<String>,
  /// 答案解析。
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub explanation: Option<String>,
}

fn non_empty(s: Option<&String>) -> Option<&str> {
  s.map(String::as_str).filter(|s| !s.is_empty())
}

impl QuestionItem {
  /// 非空 ID。
  #[must_use]
  pub fn id_str(&self) -> Option<&str> {
    non_empty(self.id.as_ref())
  }

  /// 非空题号 `J`。
  #[must_use]
  pub fn j_code(&self) -> Option<&str> {
    non_empty(self.codes.j.as_ref())
  }

  /// 非空分类码 `P`。
  #[must_use]
  pub fn p_code(&self) -> Option<&str> {
    non_empty(self.codes.p.as_ref())
  }

  /// 非空附图地址。
  #[must_use]
  pub fn image(&self) -> Option<&str> {
    non_empty(self.image_url.as_ref())
  }

  /// 是否为多选题。
  #[must_use]
  pub fn is_multiple(&self) -> bool {
    self.kind == QuestionType::Multiple
  }

  /// 作答状态存储使用的 key：优先 `id`，其次 `J`，最后退化为 `pos:{pos}`。
  #[must_use]
  pub fn answer_key(&self, pos: usize) -> String {
    self
      .id_str()
      .or_else(|| self.j_code())
      .map_or_else(|| format!("pos:{pos}"), ToOwned::to_owned)
  }

  /// 恢复考试时用于重建题目集合的稳定标识（`id` 或 `J`）。
  #[must_use]
  pub fn stable_id(&self) -> Option<String> {
    self
      .id_str()
      .or_else(|| self.j_code())
      .map(ToOwned::to_owned)
  }

  /// 关键词搜索用的小写文本：题干 + 选项 + 解析（与分类浏览页的搜索范围一致）。
  #[must_use]
  pub fn search_text(&self) -> String {
    let opts = self
      .options
      .iter()
      .map(|o| o.text.as_str())
      .collect::<Vec<_>>()
      .join(" ");
    format!(
      "{} {} {}",
      self.question,
      opts,
      self.explanation.as_deref().unwrap_or_default()
    )
    .to_lowercase()
  }

  /// 判断作答是否与标准答案完全一致（顺序无关）。
  #[must_use]
  pub fn is_answer_correct(&self, selected: &[String]) -> bool {
    same_set(selected, &self.answer_keys)
  }
}

/// 比较两个答案列表排序后是否相同。
#[must_use]
pub fn same_set(a: &[String], b: &[String]) -> bool {
  if a.len() != b.len() {
    return false;
  }
  let mut a: Vec<&str> = a.iter().map(String::as_str).collect();
  let mut b: Vec<&str> = b.iter().map(String::as_str).collect();
  a.sort_unstable();
  b.sort_unstable();
  a == b
}

/// 全站搜索用的精简题目索引条目（由 `postbuild` 从题库 JSON 提取，
/// 写入 `questions/search-index.json`，仅含题干 + 解析，不含选项与图片）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionSearchEntry {
  /// 题目 ID，如 `A-1`。
  pub id: String,
  /// 题库字母 `A` / `B` / `C`。
  pub bank: String,
  /// 题干。
  pub q: String,
  /// 解析（无解析时为空）。
  pub exp: String,
}

impl QuestionSearchEntry {
  /// 匹配文本（题干 + 解析，小写）。
  #[must_use]
  pub fn search_text(&self) -> String {
    format!("{} {}", self.q, self.exp).to_lowercase()
  }
}

/// 多选作答的错误类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiError {
  /// 漏选：只选了部分正确项，缺了某些正确项。
  Missing,
  /// 多选：选了全部正确项，但多选了错误项。
  Extra,
  /// 错选：既漏选又多选，或所选与正确答案完全不沾边。
  Wrong,
}

impl MultiError {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::Missing => "漏选",
      Self::Extra => "多选",
      Self::Wrong => "错选",
    }
  }
}

/// 多选作答的错误归类：答对或未作答返回 `None`，否则按「漏选 / 多选 / 错选」归类。
#[must_use]
pub fn multi_error_kind(selected: &[String], answer_keys: &[String]) -> Option<MultiError> {
  if selected.is_empty() || same_set(selected, answer_keys) {
    return None;
  }
  let missing = answer_keys.iter().any(|k| !selected.contains(k));
  let extra = selected.iter().any(|s| !answer_keys.contains(s));
  match (missing, extra) {
    (true, false) => Some(MultiError::Missing),
    (false, true) => Some(MultiError::Extra),
    _ => Some(MultiError::Wrong),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deserializes_dataset_format() {
    let json = r#"{"id":"A-1","codes":{"J":"LY0001","P":"1.1.1"},"question":"q","options":[{"key":"A","text":"x"}],"answer_keys":["A","C"],"type":"multiple","imageUrl":null,"explanation":"e"}"#;
    let q: QuestionItem = serde_json::from_str(json).expect("valid json");
    assert_eq!(q.answer_key(0), "A-1");
    assert!(q.is_multiple());
    assert!(q.is_answer_correct(&["C".into(), "A".into()]));
    let back = serde_json::to_string(&q).expect("serialize");
    assert_eq!(back, json);
  }

  #[test]
  fn answer_key_falls_back_to_position() {
    let q = QuestionItem {
      id: Some(String::new()),
      codes: Codes::default(),
      question: "q".into(),
      options: vec![],
      answer_keys: vec![],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    };
    assert_eq!(q.answer_key(7), "pos:7");
  }

  #[test]
  fn classifies_multi_select_errors() {
    let keys = vec!["A".to_owned(), "C".to_owned()];
    let s = |xs: &[&str]| xs.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    assert_eq!(multi_error_kind(&s(&["A", "C"]), &keys), None);
    assert_eq!(multi_error_kind(&s(&[]), &keys), None, "未作答不归因");
    assert_eq!(
      multi_error_kind(&s(&["A"]), &keys),
      Some(MultiError::Missing)
    );
    assert_eq!(
      multi_error_kind(&s(&["A", "B", "C"]), &keys),
      Some(MultiError::Extra)
    );
    assert_eq!(multi_error_kind(&s(&["B"]), &keys), Some(MultiError::Wrong));
    assert_eq!(
      multi_error_kind(&s(&["A", "B"]), &keys),
      Some(MultiError::Wrong)
    );
  }
}
