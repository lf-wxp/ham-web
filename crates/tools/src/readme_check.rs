//! README 对账：文档里那些「手写数字」和「手写清单」必须与代码一致。
//!
//! 为什么落成工具：README 上一轮审查一次踩中三处漂移 —— Worker 数量停在「十一个」（实际十二个）、
//! `build-*-worker` 清单漏掉 `afsk1200`、`cargo make check` 的构成少了五项。这类「改了代码没改
//! README」在 PR 里靠眼睛看不出，只能机械对账；和 `structure_check` / `ui_check` 是同一个思路。
//!
//! 覆盖七条（任一不符就失败）：
//! - README 里每处「N 个 Worker」都等于 `crates/*-worker` 的目录数；
//! - `Makefile.toml` 里每个 `build*-worker` 任务都在 README 出现过（任务清单不漏项）；
//! - README 里「共 N 个路径」等于 `crates/core/src/registry.rs` 的注册路径数；
//! - README 里「N 项计算器」等于 `tools_page.rs` 里 `TOOL_NAV` 的条目数；
//! - `docs/*.md` 每个文件至少被 README 或 `AGENTS.md` 链接一次（文档不孤儿）；
//! - `docs/*.md`、`ROADMAP.md` 与 `Makefile.toml` 里写的 `README「章节名」` 在 README 仍找得到；
//! - README 里引用的 `docs/images/*` 截图真实存在（重命名/删图后不会留破图）。

use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;

/// `README「章节名」` 这种跨文档引用（`docs/` 与 `Makefile.toml` 里都有）。
static SECTION_REF_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"README「([^」]+)」").expect("README 章节引用正则应能编译"));

/// 「十一 个 Worker」/「12 个 Worker」。
static WORKER_COUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?:([0-9]+)|([一二两三四五六七八九十]+))\s*个\s*Worker")
    .expect("Worker 数量正则应能编译")
});

/// Makefile 里的 `[tasks.build*-worker]`。
static BUILD_WORKER_TASK_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?m)^\[tasks\.(build[-a-z0-9]*worker)\]").expect("Worker 任务正则应能编译")
});

/// README 里的「共 180 个路径」。
static ROUTE_COUNT_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"共\s*([0-9]+)\s*个路径").expect("路由数量正则应能编译"));

/// 注册表里的 `path: "/xxx"`。
static REGISTRY_PATH_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r#"(?m)^\s*path:\s*""#).expect("注册表路径正则应能编译"));

/// README 里的「60 项计算器」。
static TOOL_COUNT_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"([0-9]+)\s*项计算器").expect("计算器数量正则应能编译"));

/// `TOOL_NAV` 数组里的 `("id", "key")` 条目（小工具的唯一清单，导航与区块都由它派生）。
static TOOL_NAV_ENTRY_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r#"(?m)^\s*\(""#).expect("工具清单条目正则应能编译"));

/// README 里的图片引用（HTML `<img src>` 与 markdown `![]()` 都算）。
static README_IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"docs/images/[A-Za-z0-9._-]+\.(?:webp|png|jpe?g|gif|svg)")
    .expect("README 图片引用正则应能编译")
});

pub fn check(root: &Path) -> Result<()> {
  let readme =
    std::fs::read_to_string(root.join("README.md")).context("failed to read README.md")?;
  let agents = std::fs::read_to_string(root.join("AGENTS.md")).unwrap_or_default();
  let makefile =
    std::fs::read_to_string(root.join("Makefile.toml")).context("failed to read Makefile.toml")?;
  let roadmap = std::fs::read_to_string(root.join("ROADMAP.md")).unwrap_or_default();
  let docs = read_docs(root)?;

  let mut failures: Vec<String> = Vec::new();
  check_worker_count(&readme, root, &mut failures)?;
  check_worker_tasks(&readme, &makefile, &mut failures);
  check_route_count(&readme, root, &mut failures)?;
  check_tool_count(&readme, root, &mut failures)?;
  check_docs_linked(&docs, &readme, &agents, &mut failures);
  check_section_refs(&docs, &readme, &makefile, &roadmap, &mut failures);
  check_images(&readme, root, &mut failures);

  if failures.is_empty() {
    println!(
      "✓ README 与代码对账通过（Worker 数量 / 任务清单 / 路由数 / 计算器数量 / 文档链接 / 章节引用 / 截图）"
    );
    return Ok(());
  }
  println!("✗ README 与代码对不上：{} 项", failures.len());
  for f in &failures {
    println!("    {f}");
  }
  println!("    → 改完代码顺手改 README；数字与清单以代码为准，别在文档里手数");
  bail!("README 对账未通过");
}

/// `crates/` 下以 `-worker` 结尾的目录数（每个对应一个 wasm Worker 产物）。
fn worker_crates(root: &Path) -> Result<usize> {
  let mut n = 0;
  for entry in std::fs::read_dir(root.join("crates")).context("failed to list crates/")? {
    let entry = entry?;
    // 只看目录：`crates/*-worker` 下都是 crate 目录，文件（含隐藏文件）不计。
    if entry.file_type()?.is_dir() && entry.file_name().to_string_lossy().ends_with("-worker") {
      n += 1;
    }
  }
  Ok(n)
}

/// 中文数字（1–99）与阿拉伯数字都转成整数；README 只会用到这个范围。
fn parse_count(raw: &str) -> Option<usize> {
  if let Ok(n) = raw.parse::<usize>() {
    return Some(n);
  }
  let digit = |c: char| match c {
    '一' => Some(1),
    '二' | '两' => Some(2),
    '三' => Some(3),
    '四' => Some(4),
    '五' => Some(5),
    '六' => Some(6),
    '七' => Some(7),
    '八' => Some(8),
    '九' => Some(9),
    _ => None,
  };
  match raw.split_once('十') {
    // 没有「十」的写法只可能是「一」～「九」；多字但无十的数字（如「两」）由首字决定。
    None => raw.chars().next().and_then(digit),
    Some((tens, ones)) => {
      let t = if tens.is_empty() {
        1
      } else {
        tens.chars().next().and_then(digit)?
      };
      let o = if ones.is_empty() {
        0
      } else {
        ones.chars().next().and_then(digit)?
      };
      Some(t * 10 + o)
    }
  }
}

fn check_worker_count(readme: &str, root: &Path, failures: &mut Vec<String>) -> Result<()> {
  let actual = worker_crates(root)?;
  for caps in WORKER_COUNT_RE.captures_iter(readme) {
    let raw = caps
      .get(1)
      .or_else(|| caps.get(2))
      .map(|m| m.as_str())
      .unwrap_or_default();
    let Some(stated) = parse_count(raw) else {
      failures.push(format!(
        "README 里的「{raw} 个 Worker」读不出数字（只支持 1–99）"
      ));
      continue;
    };
    if stated != actual {
      failures.push(format!(
        "README 写「{raw} 个 Worker」，实际 crates/*-worker 有 {actual} 个"
      ));
    }
  }
  Ok(())
}

fn check_worker_tasks(readme: &str, makefile: &str, failures: &mut Vec<String>) {
  for caps in BUILD_WORKER_TASK_RE.captures_iter(makefile) {
    let task = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
    if !readme.contains(task) {
      failures.push(format!(
        "Makefile.toml 有任务 `{task}`，但 README 的任务表没列它"
      ));
    }
  }
}

fn check_route_count(readme: &str, root: &Path, failures: &mut Vec<String>) -> Result<()> {
  let registry = std::fs::read_to_string(root.join("crates/core/src/registry.rs"))
    .context("failed to read crates/core/src/registry.rs")?;
  let actual = REGISTRY_PATH_RE.find_iter(&registry).count();
  for caps in ROUTE_COUNT_RE.captures_iter(readme) {
    let raw = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
    match raw.parse::<usize>() {
      Ok(stated) if stated == actual => {}
      _ => failures.push(format!(
        "README 写「共 {raw} 个路径」，注册表实际注册了 {actual} 条"
      )),
    }
  }
  Ok(())
}

/// README 的「N 项计算器」= `TOOL_NAV` 的条目数。
///
/// 小工具清单刻意只写在 `tools_page.rs` 的 `TOOL_NAV`（页面自己也这么注释），README 里的
/// 「60 项」是它的摘要 —— 加一个计算器很容易忘了改这里，所以按条目数对账。
fn check_tool_count(readme: &str, root: &Path, failures: &mut Vec<String>) -> Result<()> {
  let page = std::fs::read_to_string(root.join("crates/app/src/pages/tools/tools_page.rs"))
    .context("failed to read tools_page.rs")?;
  let Some(start) = page.find("const TOOL_NAV") else {
    failures
      .push("tools_page.rs 里找不到 `const TOOL_NAV`，README 的「N 项计算器」无从对账".into());
    return Ok(());
  };
  let block = &page[start..];
  let block = block.split("];").next().unwrap_or(block);
  let actual = TOOL_NAV_ENTRY_RE.find_iter(block).count();
  for caps in TOOL_COUNT_RE.captures_iter(readme) {
    let raw = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
    match raw.parse::<usize>() {
      Ok(stated) if stated == actual => {}
      _ => failures.push(format!(
        "README 写「{raw} 项计算器」，tools_page.rs 的 TOOL_NAV 实际有 {actual} 项"
      )),
    }
  }
  Ok(())
}

fn check_docs_linked(
  docs: &[(String, String)],
  readme: &str,
  agents: &str,
  failures: &mut Vec<String>,
) {
  for (name, _) in docs {
    if !readme.contains(name.as_str()) && !agents.contains(name.as_str()) {
      failures.push(format!(
        "docs/{name} 没有被 README 或 AGENTS.md 链接（文档地图里漏了它）"
      ));
    }
  }
}

/// `docs/`、`ROADMAP.md` 与 `Makefile.toml` 里 `README「章节」` 的引用必须在 README 仍能找到这段文字。
///
/// 章节改名后引用会静默失效（读者翻过去找不到），所以要求文字原样还在 —— 允许它落在标题、
/// 段落或清单图例里（`docs/code-quality.md` 就引用了「依赖 `/api/*` 的页面」这段清单说明）。
fn check_section_refs(
  docs: &[(String, String)],
  readme: &str,
  makefile: &str,
  roadmap: &str,
  failures: &mut Vec<String>,
) {
  let mut seen: Vec<String> = Vec::new();
  let sources = docs
    .iter()
    .map(|(name, text)| (format!("docs/{name}"), text.as_str()))
    .chain(std::iter::once(("Makefile.toml".to_owned(), makefile)))
    .chain(std::iter::once(("ROADMAP.md".to_owned(), roadmap)));
  for (source, text) in sources {
    for caps in SECTION_REF_RE.captures_iter(text) {
      let section = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
      let key = format!("{source}|{section}");
      if seen.contains(&key) {
        continue;
      }
      seen.push(key);
      if !readme.contains(section) {
        failures.push(format!(
          "{source} 引用了 README「{section}」，但 README 里已找不到这段文字（改名后忘了同步？）"
        ));
      }
    }
  }
}

/// README 引用的截图必须真的在 `docs/images/` 里。
///
/// 截图是 `e2e/scripts/screenshots.mjs` 生成的：重拍时改了文件名、或临时删图排查，
/// 很容易留下一条破图 —— 渲染出来只是一个碎图标，评审常看不见。
fn check_images(readme: &str, root: &Path, failures: &mut Vec<String>) {
  let mut seen: Vec<&str> = Vec::new();
  for m in README_IMAGE_RE.find_iter(readme) {
    let src = m.as_str();
    if seen.contains(&src) {
      continue;
    }
    seen.push(src);
    if !root.join(src).is_file() {
      failures.push(format!(
        "README 引用了 {src}，但文件不存在（重跑 e2e/scripts/screenshots.mjs？）"
      ));
    }
  }
}

/// 读 `docs/` 下全部 `.md`（按文件名排序，输出稳定）。
fn read_docs(root: &Path) -> Result<Vec<(String, String)>> {
  let mut docs = Vec::new();
  for entry in std::fs::read_dir(root.join("docs")).context("failed to list docs/")? {
    let entry = entry?;
    let path = entry.path();
    if !entry.file_type()?.is_file() || path.extension().is_none_or(|e| e != "md") {
      continue;
    }
    let name = entry.file_name().to_string_lossy().into_owned();
    let text = std::fs::read_to_string(&path)
      .with_context(|| format!("failed to read {}", path.display()))?;
    docs.push((name, text));
  }
  docs.sort_by(|a, b| a.0.cmp(&b.0));
  Ok(docs)
}
