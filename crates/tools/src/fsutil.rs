//! 路径与 JSON 读写工具。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// 项目内常用路径。
#[derive(Debug, Clone)]
pub struct Paths {
  /// `public/`
  pub public: PathBuf,
  /// `public/questions/`
  pub questions: PathBuf,
  /// `data/explanations.json`
  pub explanations: PathBuf,
  /// `data/glossary/`（按分类拆分的术语表目录）
  pub glossary_dir: PathBuf,
}

impl Paths {
  pub fn new(root: &Path) -> Self {
    let public = root.join("public");
    Self {
      questions: public.join("questions"),
      public,
      explanations: root.join("data").join("explanations.json"),
      glossary_dir: root.join("data").join("glossary"),
    }
  }

  /// 题库 JSON 文件路径（`A`/`B`/`C`/`full`）。
  pub fn bank_json(&self, name: &str) -> PathBuf {
    self.questions.join(format!("{name}.json"))
  }
}

/// 解析项目根目录：显式参数 > 从当前目录向上查找 `Makefile.toml` > 编译期路径。
pub fn project_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
  if let Some(root) = explicit {
    return Ok(root);
  }
  let cwd = std::env::current_dir().context("failed to read current dir")?;
  if let Some(found) = cwd.ancestors().find(|p| p.join("Makefile.toml").is_file()) {
    return Ok(found.to_path_buf());
  }
  Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
  let raw =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

/// 以 2 空格缩进写 JSON（与 `JSON.stringify(v, null, 2)` 输出一致）。
pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
  }
  let text = serde_json::to_string_pretty(value)?;
  fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}
