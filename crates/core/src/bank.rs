//! 题库类别与题库版本配置（`public/questions/config.json`）。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 考试类别 / 题库。
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Bank {
  /// A 类。
  #[default]
  A,
  /// B 类。
  B,
  /// C 类。
  C,
}

impl Bank {
  /// 全部题库，按层级顺序。
  pub const ALL: [Self; 3] = [Self::A, Self::B, Self::C];

  /// 字母表示。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::A => "A",
      Self::B => "B",
      Self::C => "C",
    }
  }

  /// 从 URL 参数解析，非法值回退为 A（与原实现一致）。
  #[must_use]
  pub fn from_param(s: Option<&str>) -> Self {
    s.and_then(|s| s.parse().ok()).unwrap_or_default()
  }

  /// 根据题目 ID 前缀判断所属题库（`B-`/`C-`，其余视为 A）。
  #[must_use]
  pub fn of_id(id: Option<&str>) -> Self {
    match id {
      Some(id) if id.starts_with("B-") => Self::B,
      Some(id) if id.starts_with("C-") => Self::C,
      _ => Self::A,
    }
  }

  /// 默认的静态 JSON 地址。
  #[must_use]
  pub fn default_url(self) -> String {
    format!("/questions/{}.json", self.as_str())
  }
}

impl fmt::Display for Bank {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

/// 解析题库字母失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBankError;

impl fmt::Display for ParseBankError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("invalid bank, expected A, B or C")
  }
}

impl std::error::Error for ParseBankError {}

impl FromStr for Bank {
  type Err = ParseBankError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "A" => Ok(Self::A),
      "B" => Ok(Self::B),
      "C" => Ok(Self::C),
      _ => Err(ParseBankError),
    }
  }
}

/// 单个题库文件信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankInfo {
  /// 题库 JSON 路径。
  pub path: String,
  /// 描述。
  pub description: String,
}

/// 三类题库文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Banks {
  /// A 类。
  #[serde(rename = "A")]
  pub a: BankInfo,
  /// B 类。
  #[serde(rename = "B")]
  pub b: BankInfo,
  /// C 类。
  #[serde(rename = "C")]
  pub c: BankInfo,
}

impl Banks {
  /// 取对应题库信息。
  #[must_use]
  pub const fn get(&self, bank: Bank) -> &BankInfo {
    match bank {
      Bank::A => &self.a,
      Bank::B => &self.b,
      Bank::C => &self.c,
    }
  }
}

/// 题库版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVersion {
  /// 版本 ID，如 `2025-10`。
  pub id: String,
  /// 名称。
  pub name: String,
  /// 描述。
  pub description: String,
  /// 是否最新版本。
  pub is_latest: bool,
  /// 题库文件。
  pub banks: Banks,
  /// 更新时间（ISO 8601）。
  pub updated_at: String,
}

impl QuestionVersion {
  /// 解析题库 JSON 地址。兼容版本目录写法：`/questions/2025-10/A.json` → `/questions/A.json`。
  #[must_use]
  pub fn resolve_url(&self, bank: Bank) -> String {
    let path = &self.banks.get(bank).path;
    if path.starts_with("/questions/") && !path.contains("/questions/202") {
      return path.clone();
    }
    if let Some(rest) = path.strip_prefix("/questions/") {
      let mut parts = rest.split('/');
      if let (Some(dir), Some(file), None) = (parts.next(), parts.next(), parts.next())
        && !dir.is_empty()
        && matches!(file, "A.json" | "B.json" | "C.json")
      {
        return format!("/questions/{file}");
      }
    }
    path.clone()
  }
}

/// 题库配置文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankConfig {
  /// 配置版本号，变化时前端会清理缓存。
  pub version: String,
  /// 最后修改时间。
  pub last_modified: String,
  /// 版本列表。
  pub versions: Vec<QuestionVersion>,
}

impl BankConfig {
  /// 按更新时间倒序排列的版本列表（ISO 8601 可直接按字符串比较）。
  #[must_use]
  pub fn sorted_versions(&self) -> Vec<QuestionVersion> {
    let mut v = self.versions.clone();
    v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    v
  }

  /// 查找版本。
  #[must_use]
  pub fn version(&self, id: &str) -> Option<&QuestionVersion> {
    self.versions.iter().find(|v| v.id == id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn version(path: &str) -> QuestionVersion {
    let info = BankInfo {
      path: path.into(),
      description: String::new(),
    };
    QuestionVersion {
      id: "v".into(),
      name: "v".into(),
      description: String::new(),
      is_latest: true,
      banks: Banks {
        a: info.clone(),
        b: info.clone(),
        c: info,
      },
      updated_at: String::new(),
    }
  }

  #[test]
  fn resolves_versioned_paths() {
    assert_eq!(
      version("/questions/A.json").resolve_url(Bank::A),
      "/questions/A.json"
    );
    assert_eq!(
      version("/questions/2025-10/B.json").resolve_url(Bank::B),
      "/questions/B.json"
    );
    assert_eq!(version("/data/x.json").resolve_url(Bank::C), "/data/x.json");
  }

  #[test]
  fn bank_parsing() {
    assert_eq!(Bank::from_param(Some("C")), Bank::C);
    assert_eq!(Bank::from_param(Some("x")), Bank::A);
    assert_eq!(Bank::of_id(Some("B-3")), Bank::B);
  }
}
