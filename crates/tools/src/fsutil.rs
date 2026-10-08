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

/// 原子写：先写同目录下的临时文件，再 `rename` 覆盖目标。
///
/// 这些 JSON 都是**手写入库**的词典（`data/i18n/{lang}/{domain}.json`、
/// `data/knowledge-i18n/{lang}/*.json`）。直接 `fs::write` 是「先截断再写」：进程在
/// 中间被 kill、磁盘满或 OOM 就会留下半截 JSON，下一次 `load_entries` 直接解析失败，
/// 而那份手写内容不可自愈（也没有 `.bak` 可回滚）。`rename` 在同一文件系统上是原子的：
/// 要么旧内容、要么新内容，读者永远看不到半个文件。
fn write_atomic(path: &Path, text: &str) -> Result<()> {
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
  }
  // 临时文件名带上进程号：并排跑两个 ham-web-tools 时不会互相覆盖。
  // 后缀刻意不是 `.json`，否则会被词典目录的 `*.json` 扫描捞进去。
  let name = path
    .file_name()
    .map_or_else(|| "out".to_owned(), |n| n.to_string_lossy().into_owned());
  let tmp = path.with_file_name(format!("{name}.tmp{}", std::process::id()));
  fs::write(&tmp, text).with_context(|| format!("failed to write {}", tmp.display()))?;
  fs::rename(&tmp, path)
    .inspect_err(|_| {
      // 覆盖失败（如跨设备）时留下临时文件比删掉更好排查，但这里清掉更干净：
      // 真正要保护的是目标文件，它此刻仍然是**完整的旧内容**。
      let _ = fs::remove_file(&tmp);
    })
    .with_context(|| format!("failed to replace {}", path.display()))
}

/// 以 2 空格缩进写 JSON（与 `JSON.stringify(v, null, 2)` 输出一致）。
///
/// 末尾补一个换行：仓库里的 JSON 都以换行结尾，缺了会让每次提交都带一个
/// 「\ No newline at end of file」噪声，也会让 `git diff` 出现整行重写。
pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
  let mut text = serde_json::to_string_pretty(value)?;
  text.push('\n');
  write_atomic(path, &text)
}

/// 与 [`write_json`] 相同，但内容未变时不落盘（返回是否写过）。
///
/// 回填译文常是「改 3 条、涉及 1 个域」，若把其余域也重写一遍，键序或空格的
/// 细微差别会带出成片无关 diff，淹没真正要评审的那几行。
pub fn write_json_if_changed<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<bool> {
  let mut text = serde_json::to_string_pretty(value)?;
  text.push('\n');
  if fs::read(path).is_ok_and(|old| old == text.as_bytes()) {
    return Ok(false);
  }
  write_atomic(path, &text)?;
  Ok(true)
}
