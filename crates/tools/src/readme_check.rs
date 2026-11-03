//! README 对账：文档里那些「手写数字」和「手写清单」必须与代码一致。
//!
//! 为什么落成工具：README 上一轮审查一次踩中三处漂移 —— Worker 数量停在「十一个」（实际十二个）、
//! `build-*-worker` 清单漏掉 `afsk1200`、`cargo make check` 的构成少了五项。这类「改了代码没改
//! README」在 PR 里靠眼睛看不出，只能机械对账；和 `structure_check` / `ui_check` 是同一个思路。
//!
//! 覆盖八条（任一不符就失败）：
//! - README 里每处「N 个 Worker」都等于 `crates/*-worker` 的目录数；
//! - `Makefile.toml` 里每个 `build*-worker` 任务都在 README 出现过（任务清单不漏项）；
//! - README 里「共 N 个路径」等于 `crates/core/src/registry.rs` 的注册路径数；
//! - README 里「N 项计算器」等于 `tools_page.rs` 里 `TOOL_NAV` 的条目数；
//! - `docs/**/*.md`（递归）每个文件都在 `docs/README.md` 里有一行链接（文档不孤儿；`docs/README.md`
//!   是**唯一索引**，README 与 `AGENTS.md` 只指向它，不各自再维护一份）；
//! - README 与 `AGENTS.md` 都链接了 `docs/README.md`（入口不断）；
//! - `docs/**/*.md`、`ROADMAP.md` 与 `Makefile.toml` 里写的 `README「章节名」` 在 README 仍找得到；
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
  let infra = read_infra(root);

  let mut failures: Vec<String> = Vec::new();
  check_worker_count(&readme, root, &mut failures)?;
  check_worker_tasks(&readme, &makefile, &mut failures);
  check_route_count(&readme, root, &mut failures)?;
  check_tool_count(&readme, root, &mut failures)?;
  check_docs_linked(&docs, &mut failures);
  check_entry_links(&readme, &agents, &mut failures);
  check_section_refs(&docs, &readme, &makefile, &roadmap, &infra, &mut failures);
  check_images(&readme, root, &mut failures);

  if failures.is_empty() {
    println!(
      "✓ README 与代码对账通过（Worker 数量 / 任务清单 / 路由数 / 计算器数量 / 文档索引 / 章节引用 / 截图）"
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

/// `docs/` 下每份文档（`docs/README.md` 自己除外）都必须在 `docs/README.md` 里有一行链接。
///
/// 为什么只认索引、不再认 README / AGENTS：三处各维护一份「文档地图」，就会各漏各的、各写各的
/// （改动前它们对同一批文档的描述已经互相对不上）。索引只有一份，漏了就是漏了。
/// 匹配的是 `](相对路径` 这种链接写法，而不是子串：`ui/modern.md` 不该被一段碰巧提到它的正文放过。
fn check_docs_linked(docs: &[(String, String)], failures: &mut Vec<String>) {
  let Some((_, index)) = docs.iter().find(|(name, _)| name == INDEX) else {
    failures.push("docs/README.md 不存在（文档索引没了）".into());
    return;
  };
  for (name, _) in docs {
    if name == INDEX {
      continue;
    }
    let linked = index.contains(&format!("]({name}")) || index.contains(&format!("](./{name}"));
    if !linked {
      failures.push(format!(
        "docs/{name} 没有被 docs/README.md 链接（索引里漏了它）"
      ));
    }
  }
}

/// README 与 `AGENTS.md` 是两个最常被点开的入口：它们必须指向 `docs/README.md`，
/// 而不是各自再列一遍文档。
fn check_entry_links(readme: &str, agents: &str, failures: &mut Vec<String>) {
  for (label, text) in [("README.md", readme), ("AGENTS.md", agents)] {
    if !text.contains("docs/README.md") {
      failures.push(format!(
        "{label} 没有链接 docs/README.md（入口指不到唯一索引）"
      ));
    }
  }
}

/// `docs/`、`ROADMAP.md` 与 `Makefile.toml` 里 `README「章节」` 的引用必须在 README 仍能找到这段文字。
///
/// 章节改名后引用会静默失效（读者翻过去找不到），所以要求文字原样还在 —— 允许它落在标题、
/// 段落或清单图例里（`docs/engineering/code-quality.md` 就引用了「依赖 `/api/*` 的页面」这段清单说明）。
fn check_section_refs(
  docs: &[(String, String)],
  readme: &str,
  makefile: &str,
  roadmap: &str,
  infra: &[(String, String)],
  failures: &mut Vec<String>,
) {
  let sources = docs
    .iter()
    .map(|(name, text)| (format!("docs/{name}"), text.as_str()))
    .chain(std::iter::once(("Makefile.toml".to_owned(), makefile)))
    .chain(std::iter::once(("ROADMAP.md".to_owned(), roadmap)))
    .chain(
      infra
        .iter()
        .map(|(name, text)| (name.clone(), text.as_str())),
    );
  failures.extend(dangling_section_refs(sources, readme));
}

/// 在若干 `(来源名, 文本)` 里找出「`README「章节」` 但 README 里已没有这段文字」的引用（每个来源 × 章节只报一次）。
fn dangling_section_refs<'a>(
  sources: impl IntoIterator<Item = (String, &'a str)>,
  readme: &str,
) -> Vec<String> {
  let mut seen: Vec<String> = Vec::new();
  let mut out = Vec::new();
  for (source, text) in sources {
    for caps in SECTION_REF_RE.captures_iter(text) {
      let section = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
      let key = format!("{source}|{section}");
      if seen.contains(&key) {
        continue;
      }
      seen.push(key);
      if !readme.contains(section) {
        out.push(format!(
          "{source} 引用了 README「{section}」，但 README 里已找不到这段文字（改名后忘了同步？）"
        ));
      }
    }
  }
  out
}

/// `Dockerfile` 与 `.github/workflows/*.yml`：它们的注释里也会写「见 README「某章节」」。
///
/// 为什么要读：README 改成索引时搬走了章节，这两处的引用没人同步（`Dockerfile` 与 `build-docker.yml`
/// 各留了一条死引用），而只扫 `docs/` / `ROADMAP.md` / `Makefile.toml` 的检查看不见它们。
/// 读不到的文件静默跳过（比如没有 `.github/` 的发行包），因为这是「额外来源」，不是必需输入。
fn read_infra(root: &Path) -> Vec<(String, String)> {
  let mut out = Vec::new();
  if let Ok(text) = std::fs::read_to_string(root.join("Dockerfile")) {
    out.push(("Dockerfile".to_owned(), text));
  }
  if let Ok(entries) = std::fs::read_dir(root.join(".github/workflows")) {
    let mut paths: Vec<_> = entries
      .flatten()
      .map(|e| e.path())
      .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
      .collect();
    paths.sort();
    for path in paths {
      if let (Some(name), Ok(text)) = (
        path.file_name().and_then(|n| n.to_str()),
        std::fs::read_to_string(&path),
      ) {
        out.push((format!(".github/workflows/{name}"), text));
      }
    }
  }
  out
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

/// 文档索引的文件名（相对 `docs/`）。
const INDEX: &str = "README.md";

/// 递归读 `docs/` 下全部 `.md`，键是相对 `docs/` 的路径（`ui/modern.md`，分隔符统一为 `/`），
/// 按路径排序，输出稳定。
///
/// 必须递归：`docs/` 按读者问题分了子目录，若只扫顶层，移进子目录的文档就整体逃出了
/// 「不孤儿」检查 —— 而且静默（扫不到 = 不检查 = 不报错）。
fn read_docs(root: &Path) -> Result<Vec<(String, String)>> {
  let base = root.join("docs");
  let mut docs = Vec::new();
  for entry in walkdir::WalkDir::new(&base).sort_by_file_name() {
    let entry = entry.context("failed to walk docs/")?;
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "md") {
      continue;
    }
    let name = path
      .strip_prefix(&base)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let text = std::fs::read_to_string(path)
      .with_context(|| format!("failed to read {}", path.display()))?;
    docs.push((name, text));
  }
  docs.sort_by(|a, b| a.0.cmp(&b.0));
  Ok(docs)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn doc(name: &str) -> (String, String) {
    (name.to_owned(), String::new())
  }

  fn index_doc(text: &str) -> (String, String) {
    (INDEX.to_owned(), text.to_owned())
  }

  #[test]
  fn every_doc_must_be_in_the_index_including_subdirectories() {
    let docs = vec![
      index_doc("| [现代风](ui/modern.md) | 规范 |\n| [任务](./engineering/tasks.md) | 清单 |"),
      doc("ui/modern.md"),
      doc("engineering/tasks.md"),
      doc("data/i18n.md"),
    ];
    let mut failures = Vec::new();
    check_docs_linked(&docs, &mut failures);
    assert_eq!(
      failures,
      vec!["docs/data/i18n.md 没有被 docs/README.md 链接（索引里漏了它）".to_owned()],
      "`./` 开头与不带 `./` 的写法都算链接；漏的那份必须被点名"
    );
  }

  #[test]
  fn a_prose_mention_is_not_a_link() {
    // 正文里提到了文件名但没有链接：不算「索引里有」。
    let docs = vec![index_doc("详见 ui/modern.md 的第二节"), doc("ui/modern.md")];
    let mut failures = Vec::new();
    check_docs_linked(&docs, &mut failures);
    assert_eq!(failures.len(), 1, "只有 `](路径` 才算链接，提到名字不算");
  }

  #[test]
  fn a_longer_name_does_not_vouch_for_a_shorter_one() {
    // `ui/modern-classes.md` 的链接不能顺带让 `ui/modern.md` 过关（前缀 / 子串陷阱）。
    let docs = vec![
      index_doc("[类名](ui/modern-classes.md)"),
      doc("ui/modern.md"),
      doc("ui/modern-classes.md"),
    ];
    let mut failures = Vec::new();
    check_docs_linked(&docs, &mut failures);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("ui/modern.md"), "{failures:?}");
  }

  #[test]
  fn dangling_readme_section_refs_are_reported_once_per_source() {
    let readme = "## 常见问题\n\n正文";
    let sources = vec![
      (
        "Dockerfile".to_owned(),
        "# 见 README「构建耗时与缓存」，再见 README「构建耗时与缓存」",
      ),
      (".github/x.yml".to_owned(), "# README「常见问题」"),
    ];
    let out = dangling_section_refs(sources, readme);
    assert_eq!(
      out,
      vec![
        "Dockerfile 引用了 README「构建耗时与缓存」，但 README 里已找不到这段文字（改名后忘了同步？）"
          .to_owned()
      ],
      "同一来源同一章节只报一次；README 里还在的章节不报"
    );
  }

  #[test]
  fn a_missing_index_is_a_failure_not_a_pass() {
    let mut failures = Vec::new();
    check_docs_linked(&[doc("ui/modern.md")], &mut failures);
    assert_eq!(
      failures,
      vec!["docs/README.md 不存在（文档索引没了）".to_owned()]
    );
  }

  #[test]
  fn the_index_itself_is_exempt() {
    let mut failures = Vec::new();
    check_docs_linked(&[index_doc("")], &mut failures);
    assert!(failures.is_empty());
  }

  #[test]
  fn readme_and_agents_must_point_at_the_index() {
    let mut failures = Vec::new();
    check_entry_links("见 [索引](docs/README.md)", "没有任何链接", &mut failures);
    assert_eq!(
      failures,
      vec!["AGENTS.md 没有链接 docs/README.md（入口指不到唯一索引）".to_owned()]
    );
  }
}
