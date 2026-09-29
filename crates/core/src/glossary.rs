//! 术语表（`data/glossary.json`）：业余无线电常用术语、英文缩写与通俗解释。
//!
//! 文件为 JSON 对象，key 为术语（中文名称或缩写本身），value 支持两种写法：
//!
//! ```json
//! {
//!   "驻波比": {
//!     "abbr": "SWR",
//!     "en": "Standing Wave Ratio",
//!     "category": "天线",
//!     "desc": "衡量天线和连接线配合得好不好的指标",
//!     "aliases": ["电压驻波比"],
//!     "see": "反射系数",
//!     "inject": true
//!   },
//!   "旧写法": "只有解释文本，等价于 { \"desc\": …, \"inject\": true }"
//! }
//! ```
//!
//! - `category` 取一级分类 key（见 [`crate::categories::TOP_CATEGORIES`]），缺省归入「其他」；
//! - `inject` 为 `true` 的词条会被 `enhance-explanations` 注入到解析中首次出现的位置，默认 `false`；
//! - 词条顺序即文件顺序（术语长度相同时的注入优先级依赖该顺序）。

use std::collections::HashSet;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::categories::top_category;

/// 未指定或未知分类时的归类名称。
pub const OTHER_CATEGORY: &str = "其他";

/// 一个术语条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntry {
  /// 术语（中文名称或缩写本身）。
  pub term: String,
  /// 英文缩写，如 `SWR`。
  pub abbr: Option<String>,
  /// 英文全称，如 `Standing Wave Ratio`。
  pub en: Option<String>,
  /// 一级分类 key，如 `天线`。
  pub category: Option<String>,
  /// 通俗解释。
  pub desc: String,
  /// 别名（仅用于搜索与展示）。
  pub aliases: Vec<String>,
  /// 参见的其他术语。
  pub see: Option<String>,
  /// 是否用于解析术语注入。
  pub inject: bool,
}

impl GlossaryEntry {
  /// 英文缩写：优先 `abbr`，术语本身为 ASCII（如 `QRM`、`73`）时返回术语。
  #[must_use]
  pub fn abbreviation(&self) -> Option<&str> {
    self
      .abbr
      .as_deref()
      .or_else(|| self.term.is_ascii().then_some(self.term.as_str()))
  }

  /// 分类 key，未指定时为 [`OTHER_CATEGORY`]。
  #[must_use]
  pub fn category_key(&self) -> &str {
    self
      .category
      .as_deref()
      .filter(|c| top_category(c).is_some())
      .unwrap_or(OTHER_CATEGORY)
  }

  /// 搜索匹配度（越小越相关），不匹配返回 `None`。`query_lower` 需已 trim 并转小写。
  ///
  /// 0：术语/缩写/别名完全相同；1：前缀匹配；2：包含于术语/缩写/别名/英文全称；3：包含于解释。
  #[must_use]
  pub fn match_rank(&self, query_lower: &str) -> Option<u8> {
    if query_lower.is_empty() {
      return Some(0);
    }
    let names = std::iter::once(self.term.as_str())
      .chain(self.abbr.as_deref())
      .chain(self.aliases.iter().map(String::as_str));
    let mut best: Option<u8> = None;
    for name in names {
      let name = name.to_lowercase();
      let rank = if name == query_lower {
        0
      } else if name.starts_with(query_lower) {
        1
      } else if name.contains(query_lower) {
        2
      } else {
        continue;
      };
      best = Some(best.map_or(rank, |b| b.min(rank)));
    }
    best
      .or_else(|| {
        self
          .en
          .as_deref()
          .filter(|en| en.to_lowercase().contains(query_lower))
          .map(|_| 2)
      })
      .or_else(|| self.desc.to_lowercase().contains(query_lower).then_some(3))
  }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
  Text(String),
  Full(RawFields),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFields {
  #[serde(default)]
  abbr: Option<String>,
  #[serde(default)]
  en: Option<String>,
  #[serde(default)]
  category: Option<String>,
  desc: String,
  #[serde(default)]
  aliases: Vec<String>,
  #[serde(default)]
  see: Option<String>,
  #[serde(default)]
  inject: bool,
}

impl RawEntry {
  fn into_entry(self, term: String) -> GlossaryEntry {
    match self {
      Self::Text(desc) => GlossaryEntry {
        term,
        abbr: None,
        en: None,
        category: None,
        desc,
        aliases: Vec::new(),
        see: None,
        inject: true,
      },
      Self::Full(f) => GlossaryEntry {
        term,
        abbr: f.abbr,
        en: f.en,
        category: f.category,
        desc: f.desc,
        aliases: f.aliases,
        see: f.see,
        inject: f.inject,
      },
    }
  }
}

/// 术语表（保持文件中的词条顺序）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Glossary {
  entries: Vec<GlossaryEntry>,
}

impl<'de> Deserialize<'de> for Glossary {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct GlossaryVisitor;

    impl<'de> Visitor<'de> for GlossaryVisitor {
      type Value = Glossary;

      fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object of `term: description | { desc, abbr, en, category, … }`")
      }

      fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Glossary, A::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((term, raw)) = map.next_entry::<String, RawEntry>()? {
          entries.push(raw.into_entry(term));
        }
        Ok(Glossary { entries })
      }
    }

    deserializer.deserialize_map(GlossaryVisitor)
  }
}

const FULL_WIDTH_PARENS: [char; 2] = ['（', '）'];

impl Glossary {
  /// 全部词条。
  #[must_use]
  pub fn entries(&self) -> &[GlossaryEntry] {
    &self.entries
  }

  /// 按术语查找。
  #[must_use]
  pub fn get(&self, term: &str) -> Option<&GlossaryEntry> {
    self.entries.iter().find(|e| e.term == term)
  }

  /// 用于解析注入的 `(术语, 解释)` 列表，保持文件顺序。
  #[must_use]
  pub fn inject_pairs(&self) -> Vec<(String, String)> {
    self
      .entries
      .iter()
      .filter(|e| e.inject)
      .map(|e| (e.term.clone(), e.desc.clone()))
      .collect()
  }

  /// 校验数据，返回问题列表（为空表示通过）。
  #[must_use]
  pub fn validate(&self) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    let terms: HashSet<&str> = self.entries.iter().map(|e| e.term.as_str()).collect();

    for e in &self.entries {
      let t = &e.term;
      if t.trim().is_empty() || t.trim() != t {
        problems.push(format!("术语「{t}」为空或首尾含空白"));
      }
      if !seen.insert(t.as_str()) {
        problems.push(format!("术语「{t}」重复"));
      }
      if e.desc.trim().is_empty() {
        problems.push(format!("「{t}」缺少解释 desc"));
      }
      if e.desc.contains(FULL_WIDTH_PARENS) {
        problems.push(format!("「{t}」的解释含全角括号，会与注入格式冲突"));
      }
      if e.inject && e.desc.contains(['(', ')']) {
        problems.push(format!("「{t}」参与注入，解释中不能含括号"));
      }
      if let Some(c) = e.category.as_deref()
        && top_category(c).is_none()
      {
        problems.push(format!("「{t}」的分类「{c}」不存在"));
      }
      if let Some(see) = &e.see
        && (see == t || !terms.contains(see.as_str()))
      {
        problems.push(format!("「{t}」参见的「{see}」不存在"));
      }
      for (field, value) in [("abbr", &e.abbr), ("en", &e.en)] {
        if value
          .as_deref()
          .is_some_and(|v| v.trim().is_empty() || v.trim() != v)
        {
          problems.push(format!("「{t}」的 {field} 为空或首尾含空白"));
        }
      }
    }
    problems
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse(json: &str) -> Glossary {
    serde_json::from_str(json).expect("valid glossary")
  }

  #[test]
  fn supports_legacy_and_structured_entries_in_order() {
    let g = parse(r#"{"b": "旧写法", "a": {"abbr": "A", "category": "天线", "desc": "新写法"}}"#);
    let terms: Vec<_> = g.entries().iter().map(|e| e.term.as_str()).collect();
    assert_eq!(terms, ["b", "a"]);
    assert!(g.entries()[0].inject);
    assert!(!g.entries()[1].inject);
    assert_eq!(
      g.inject_pairs(),
      vec![("b".to_owned(), "旧写法".to_owned())]
    );
    assert_eq!(g.get("a").and_then(GlossaryEntry::abbreviation), Some("A"));
  }

  #[test]
  fn ranks_matches() {
    let g = parse(
      r#"{"驻波比": {"abbr": "SWR", "en": "Standing Wave Ratio", "category": "天线", "desc": "匹配程度"}}"#,
    );
    let e = &g.entries()[0];
    assert_eq!(e.match_rank("swr"), Some(0));
    assert_eq!(e.match_rank("驻波"), Some(1));
    assert_eq!(e.match_rank("wave"), Some(2));
    assert_eq!(e.match_rank("匹配"), Some(3));
    assert_eq!(e.match_rank("天调"), None);
  }

  #[test]
  fn validation_reports_problems() {
    let g = parse(r#"{"x": {"category": "不存在", "desc": "（坏）", "see": "y"}}"#);
    assert_eq!(g.validate().len(), 3);
  }

  #[test]
  fn project_glossary_is_valid() {
    let g: Glossary = serde_json::from_str(include_str!("../../../data/glossary.json"))
      .expect("data/glossary.json");
    let problems = g.validate();
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(g.entries().len() >= 300);
    let uncategorized: Vec<_> = g
      .entries()
      .iter()
      .filter(|e| e.category.is_none())
      .map(|e| &e.term)
      .collect();
    assert!(uncategorized.is_empty(), "缺少分类：{uncategorized:?}");
  }
}
