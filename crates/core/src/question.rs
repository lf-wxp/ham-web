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
  ///
  /// 有稳定标识时它就是 [`stable_id`](Self::stable_id)，所以不必再写
  /// `stable_id().unwrap_or_else(|| answer_key(pos))`（恒等于本函数）。
  /// 区别只在兜底：位置键 `pos:N` 随列表顺序变，**不能当持久化标识**（收藏、笔记）用；
  /// 需要「没有稳定编号就不存」的场景请直接用 `stable_id`。
  #[must_use]
  pub fn answer_key(&self, pos: usize) -> String {
    self
      .id_str()
      .or_else(|| self.j_code())
      .map_or_else(|| format!("pos:{pos}"), ToOwned::to_owned)
  }

  /// 恢复考试时用于重建题目集合的稳定标识（`id` 或 `J`）；两者都缺时为 `None`。
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

/// 题目卡片的「答案行」该不该显示。
///
/// 为什么单独成一个判定：练习页的原则是「作答后才给答案」，但多选题只要勾上第一个框
/// 就算「已作答」，答案行当场把整套正确答案亮出来 —— 后面的框照着抄就行，练习失去意义。
/// 多选题因此要先点「检查答案」（`needs_check`）才算作答完毕。判定放在 core 里带单测，
/// 视图只负责把几个信号喂进来。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnswerGate {
  /// 调用方允许显示答案（练习页的开关；考试进行中是 `false`）。
  pub allowed: bool,
  /// 未作答也直接显示（交卷后的回顾）。
  pub reveal_unanswered: bool,
  /// 用户主动点了「显示正确答案」。
  pub revealed: bool,
  /// 已经选了至少一项。
  pub answered: bool,
  /// 这道题要求先点「检查答案」（多选 + 调用方开启了该模式）。
  pub needs_check: bool,
  /// 已经点过「检查答案」。
  pub checked: bool,
}

impl AnswerGate {
  /// 答案行是否可见。
  #[must_use]
  pub const fn visible(self) -> bool {
    if !self.allowed {
      return false;
    }
    if self.reveal_unanswered || self.revealed {
      return true;
    }
    self.answered && (!self.needs_check || self.checked)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn answer_gate_hides_answer_until_the_question_is_really_answered() {
    let base = AnswerGate {
      allowed: true,
      ..AnswerGate::default()
    };
    assert!(!base.visible(), "没作答：不给答案");
    assert!(
      AnswerGate {
        answered: true,
        ..base
      }
      .visible(),
      "单选选了就判分"
    );
  }

  #[test]
  fn answer_gate_makes_multi_select_wait_for_the_check_button() {
    let multi = AnswerGate {
      allowed: true,
      answered: true,
      needs_check: true,
      ..AnswerGate::default()
    };
    assert!(!multi.visible(), "勾上第一个框不能亮出整套答案");
    assert!(
      AnswerGate {
        checked: true,
        ..multi
      }
      .visible()
    );
  }

  #[test]
  fn answer_gate_respects_the_global_switch_and_explicit_reveals() {
    // 考试进行中：什么都不给，哪怕交卷后才有的开关被误设
    let exam = AnswerGate {
      allowed: false,
      reveal_unanswered: true,
      revealed: true,
      answered: true,
      needs_check: false,
      checked: true,
    };
    assert!(!exam.visible());
    // 主动「显示正确答案」与交卷回顾：不要求作答，也不要求检查
    let peek = AnswerGate {
      allowed: true,
      revealed: true,
      needs_check: true,
      ..AnswerGate::default()
    };
    assert!(peek.visible());
    let review = AnswerGate {
      allowed: true,
      reveal_unanswered: true,
      ..AnswerGate::default()
    };
    assert!(review.visible());
  }

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

  /// 钉住 `answer_key` 与 `stable_id` 的关系：调用点据此把
  /// `stable_id().unwrap_or_else(|| answer_key(i))` 化简成 `answer_key(i)`，
  /// 而「没有稳定编号就不给收藏入口」依赖 `stable_id` 在两码皆缺时为 `None`。
  #[test]
  fn answer_key_agrees_with_stable_id_and_only_differs_in_the_fallback() {
    let blank = |id: Option<&str>, j: Option<&str>| QuestionItem {
      id: id.map(str::to_owned),
      codes: Codes {
        j: j.map(str::to_owned),
        p: None,
      },
      question: "q".into(),
      options: vec![],
      answer_keys: vec![],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: None,
    };

    let with_id = blank(Some("A-1"), Some("LY0001"));
    assert_eq!(with_id.stable_id().as_deref(), Some("A-1"), "id 优先于 J");
    assert_eq!(with_id.answer_key(9), "A-1");

    let only_j = blank(Some(""), Some("LY0001"));
    assert_eq!(only_j.stable_id().as_deref(), Some("LY0001"), "空 id 不算");
    assert_eq!(only_j.answer_key(9), "LY0001");

    let neither = blank(None, Some(""));
    assert_eq!(neither.stable_id(), None, "两码皆缺：没有稳定标识");
    assert_eq!(
      neither.answer_key(9),
      "pos:9",
      "answer_key 才会退化为位置键"
    );

    for q in [&with_id, &only_j, &neither] {
      assert_eq!(
        q.stable_id().unwrap_or_else(|| q.answer_key(3)),
        q.answer_key(3),
        "两种写法恒等"
      );
    }
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
