//! 结构与架构的机械守卫：把 `docs/code-quality.md` 里「应为 0 / 单一来源」的几条搬进
//! `cargo make check`。
//!
//! 为什么落成工具：分层穿透（core 碰浏览器 API、crate 反向依赖 app/server）、一文件多组件、
//! 页面手写吸顶偏移、散落的 `alert()` 这些「结构回潮」靠人记得手跑 grep 不可靠 ——
//! 和 `ui_check` 把界面规范收口是同一个道理。
//!
//! 覆盖 `docs/code-quality.md` 的这几条（编号沿用该文档的检查项 ID）：
//! - C3.1 一文件一组件：一个文件里出现两个以上「返回视图的函数」（`-> impl IntoView / AnyView`）；
//! - C2.3 页内手写吸顶偏移：`sticky top-[…]` 出现在页面里（第二根吸顶栏唯一来源是 StickyPanel）；
//! - C6.1 散落的 `alert()`：反馈应走 Toast / MessageDialog；
//! - A1.1 core 依赖浏览器 API：`web_sys` / `js_sys` / `wasm_bindgen` 出现在 core 源码；
//! - A1.3 crate 反向依赖 app/server：任何 crate 的依赖里出现 ham-web-app / ham-web-server；
//! - A2.1 数据显示层带色值：`crates/core` 里出现 `"#rrggbb"` 字面量（分类色这类「标识色」
//!   必须只给档位，色值归界面层的 `--cat-*`，否则 11 套配色方案会失效）；
//! - 运行时 panic 宏：`todo!()` / `unimplemented!()` 不应留到提交；
//! - 知识库译文词典挂载：页面用了 `kt_tracked` / `core_text_tracked` 渲染 `crates/core` 的中文原文，
//!   就必须在该页面（或其目录）里调 `use_knowledge_i18n()` 或走 `KnowledgePage` 外壳，
//!   否则以英文 / 西语直接打开该页时词典永远不来、文案静默停在中文。
//!
//! 判定口径：每行先剥掉 `//` 之后的内容，`/*` 与 `*` 开头的行整行跳过（历史说明不算违规），
//! 与 `ui_check::code_part` 同语义 —— 这比手工 grep 更严格地「只看代码」，`qsl_image.rs` 里
//! 注释提到的 `web_sys::File::size()` 因此不会误报。

use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;
use walkdir::WalkDir;

/// 每条规则最多打印多少条命中，避免一次刷屏。
const MAX_REPORT: usize = 10;

/// 一条逐行守卫规则。
struct LineRule {
  /// 规则名（输出用）
  name: &'static str,
  /// 扫描根（相对项目根目录）
  roots: &'static [&'static str],
  /// 相对路径里含这些片段就跳过
  skip: &'static [&'static str],
  /// 命中即违规
  hits: fn(&str) -> bool,
  /// 整改提示
  advice: &'static str,
}

const RULES: &[LineRule] = &[
  LineRule {
    name: "页面手写吸顶偏移（`sticky top-[…]`）",
    roots: &["crates/app/src/pages"],
    skip: &[],
    hits: has_sticky_top,
    advice: "页内第二根吸顶栏走 common::StickyPanel；页头吸顶骨架是 PAGE_HEADER_BAR 的唯一来源，页面里不要手写固定偏移",
  },
  LineRule {
    name: "散落的 `alert()`",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_alert,
    advice: "轻量反馈走 Toast，需确认的走 MessageDialog，不要用浏览器原生 alert",
  },
  LineRule {
    name: "core 依赖浏览器 API（`web_sys` / `js_sys` / `wasm_bindgen`）",
    roots: &["crates/core/src"],
    skip: &[],
    hits: has_browser_api,
    advice: "不依赖浏览器的逻辑放 ham-web-core；这三样只能在 crates/app 与各 worker 里出现",
  },
  // 色值白名单：`color.rs` / `color_scheme.rs` / 像素调色板工具本来就是「定义颜色」的地方。
  LineRule {
    name: "数据显示层带色值（`crates/core` 里的 `\"#rrggbb\"`）",
    roots: &["crates/core/src"],
    skip: &["color.rs", "color_scheme.rs", "/pixel/"],
    hits: has_hex_color,
    advice: "分类标识色只给档位（`CategoryTone`），色值在 `crates/app/src/tone.rs` 映射到 `--cat-*`；数据层写死色值会让 11 套配色方案失效",
  },
  // 本文件自身需要写 `todo!()` / `unimplemented!()` 这些词来识别它们，跳过自身避免误报。
  LineRule {
    name: "运行时 panic 宏（`todo!()` / `unimplemented!()`）",
    roots: &["crates"],
    skip: &["structure_check.rs"],
    hits: has_panic_macro,
    advice: "未完成 / 不可达用 Result / Option 表达，别把 todo!() / unimplemented!() 留到提交",
  },
];

fn has_sticky_top(line: &str) -> bool {
  line.contains("sticky top-[")
}

/// 浏览器原生 `alert(`:前面不能紧挨着标识符字符,否则 `show_alert(` / `raise_alert(` 这类
/// 自定义函数会被误判;`window.alert(` 与行首的 `alert(` 仍要命中。
fn has_alert(line: &str) -> bool {
  line.match_indices("alert(").any(|(i, _)| {
    line[..i]
      .chars()
      .next_back()
      .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
  })
}

fn has_browser_api(line: &str) -> bool {
  ["web_sys", "js_sys", "wasm_bindgen"]
    .iter()
    .any(|n| line.contains(n))
}

/// `"#rrggbb"` / `"#rgb"` 字面量（色值）：大小写都算，`"#` 后面至少 3 位十六进制。
fn has_hex_color(line: &str) -> bool {
  let bytes = line.as_bytes();
  for i in 0..bytes.len().saturating_sub(3) {
    if bytes[i] == b'"' && bytes[i + 1] == b'#' {
      let hex = &line[i + 2..];
      let n = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
      if matches!(n, 3 | 4 | 6 | 8) && hex[n..].starts_with('"') {
        return true;
      }
    }
  }
  false
}

fn has_panic_macro(line: &str) -> bool {
  line.contains("todo!()") || line.contains("unimplemented!()")
}

/// 剥掉行注释，得到「参与判定的代码部分」。与 `ui_check::code_part` 同语义。
///
/// `/*` 与 `*` 开头的整行按块注释续行处理（`*slot = …` 这种解引用语句因此不参与扫描，
/// 代价是漏报 —— 它本来也不该出现被守卫的字符串）。字符串里含 `//`（URL）的行会被截断，
/// 只可能漏报、不会误报。
fn code_part(line: &str) -> &str {
  let trimmed = line.trim_start();
  if trimmed.starts_with("/*") || trimmed.starts_with('*') {
    return "";
  }
  match line.find("//") {
    Some(i) => &line[..i],
    None => line,
  }
}

/// 扫描若干目录下所有 `.rs`，对每行（剥注释后）跑 `hits`，返回 (命中总数, 前 MAX_REPORT 条证据)。
fn scan(
  roots: &[&str],
  root: &Path,
  skip: &[&str],
  hits: fn(&str) -> bool,
) -> (usize, Vec<String>) {
  let mut total = 0usize;
  let mut report: Vec<String> = Vec::new();
  for dir in roots {
    for entry in WalkDir::new(root.join(dir))
      .sort_by_file_name()
      .into_iter()
      .filter_map(Result::ok)
    {
      let path = entry.path();
      if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
        continue;
      }
      let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
      if skip.iter().any(|s| rel.contains(s)) {
        continue;
      }
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for (i, line) in text.lines().enumerate() {
        let code = code_part(line);
        if code.is_empty() || !hits(code) {
          continue;
        }
        total += 1;
        if report.len() < MAX_REPORT {
          report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
        }
      }
    }
  }
  (total, report)
}

/// 跑全部逐行规则，返回未通过规则的名称列表。
fn run_line_rules(root: &Path) -> Vec<String> {
  let mut failed = Vec::new();
  for rule in RULES {
    let (total, report) = scan(rule.roots, root, rule.skip, rule.hits);
    if total == 0 {
      println!("✓ {}", rule.name);
    } else {
      println!("✗ {}：{total} 处", rule.name);
      for line in &report {
        println!("{line}");
      }
      if total > report.len() {
        println!("    …（另有 {} 处，先修上面这些）", total - report.len());
      }
      println!("    → {}", rule.advice);
      failed.push(rule.name.to_owned());
    }
  }
  failed
}

/// 「一个文件只放一个组件」的正则：`-> impl IntoView` / `-> AnyView` / `-> IntoView`。
///
/// 与 `AGENTS.md` 的自检命令同口径（`impl` 后的可选路径 + 两个返回类型），逐行应用
/// （`\s` 会匹配换行，逐行跑才不会跨行误匹配）。
static VIEW_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"->\s*(impl\s+[A-Za-z_:]*\s*)?(IntoView|AnyView)").expect("一文件一组件正则应能编译")
});

/// C3.1：每个 `.rs` 文件里「返回视图的函数」最多一个，多一个就该拆文件。
fn check_one_component_per_file(root: &Path) -> bool {
  let mut report: Vec<String> = Vec::new();
  for entry in WalkDir::new(root.join("crates/app/src"))
    .sort_by_file_name()
    .into_iter()
    .filter_map(Result::ok)
  {
    let path = entry.path();
    if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
      continue;
    }
    let rel = path
      .strip_prefix(root)
      .unwrap_or(path)
      .to_string_lossy()
      .replace('\\', "/");
    let Ok(text) = std::fs::read_to_string(path) else {
      continue;
    };
    let count: usize = text
      .lines()
      .map(|l| VIEW_FN_RE.find_iter(code_part(l)).count())
      .sum();
    if count > 1 {
      report.push(format!("    {rel}: {count} 个返回视图的函数"));
    }
  }

  if report.is_empty() {
    println!("✓ 一文件一组件：每个文件最多一个返回视图的函数");
    return true;
  }
  println!("✗ 一文件一组件：{} 个文件超标", report.len());
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 一个文件只放一个组件（含任何 `-> impl IntoView / AnyView` 的视图函数）；主从组件用文件夹模块（mod.rs + 子文件）"
  );
  false
}

/// A1.3：任何 crate 的依赖都不得反向指向 `ham-web-app` / `ham-web-server`。
///
/// 只看「依赖 key」：行首（剥注释、去空白后）以 `ham-web-app` / `ham-web-server` 开头的才算，
/// 所以 `crates/app/Cargo.toml` 里 `name = "ham-web-app"` 这类 `[package]` 声明不会被误判。
fn check_no_reverse_deps(root: &Path) -> bool {
  let mut report: Vec<String> = Vec::new();
  let Ok(entries) = std::fs::read_dir(root.join("crates")) else {
    return true;
  };
  for entry in entries.flatten() {
    let dir = entry.path();
    if !dir.is_dir() {
      continue;
    }
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
      continue;
    }
    let crate_name = dir
      .file_name()
      .unwrap_or_default()
      .to_string_lossy()
      .to_string();
    let Ok(text) = std::fs::read_to_string(&manifest) else {
      continue;
    };
    for (i, line) in text.lines().enumerate() {
      let key = code_part(line).trim_start();
      if key.starts_with("ham-web-app") || key.starts_with("ham-web-server") {
        report.push(format!(
          "    crates/{crate_name}/Cargo.toml:{}  {key}",
          i + 1
        ));
      }
    }
  }

  if report.is_empty() {
    println!("✓ crate 依赖方向：无任何 crate 反向依赖 ham-web-app / ham-web-server");
    return true;
  }
  println!("✗ crate 依赖方向：{} 处反向依赖", report.len());
  for line in &report {
    println!("{line}");
  }
  println!("    → 分层依赖只能从上往下：core / 解码 crate / worker 不得依赖 app / server");
  false
}

/// 这一行是不是在用知识库译文（`crates/core` 中文原文 → 当前语言）。
fn uses_knowledge_text(code: &str) -> bool {
  code.contains("kt_tracked(") || code.contains("core_text_tracked(")
}

/// 这一行是不是在保证译文词典会被拉取：直接调 `use_knowledge_i18n`，或走自带它的 `KnowledgePage` 外壳。
fn loads_knowledge_dict(code: &str) -> bool {
  code.contains("use_knowledge_i18n") || code.contains("KnowledgePage")
}

/// A3.1：用了知识库译文的页面必须挂词典拉取。
///
/// 以 `pages/` 下的一级条目为单位（单文件页面看这个文件；目录页面看整个目录——子组件的词典由
/// 同目录的页面主体负责拉取）。`components/` 里的共享组件挂在各页面之下，没有「页面」这个边界可判，
/// 不在这条规则里。
fn check_knowledge_dict_is_loaded(root: &Path) -> bool {
  let pages = root.join("crates/app/src/pages");
  let Ok(entries) = std::fs::read_dir(&pages) else {
    return true;
  };
  let mut units: Vec<_> = entries.flatten().map(|e| e.path()).collect();
  units.sort();

  let mut report: Vec<String> = Vec::new();
  for unit in units {
    let (mut uses, mut loads) = (false, false);
    for entry in WalkDir::new(&unit).into_iter().filter_map(Result::ok) {
      let path = entry.path();
      if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "rs") {
        continue;
      }
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for line in text.lines() {
        let code = code_part(line);
        uses |= uses_knowledge_text(code);
        loads |= loads_knowledge_dict(code);
      }
    }
    if uses && !loads {
      let rel = unit
        .strip_prefix(root)
        .unwrap_or(&unit)
        .to_string_lossy()
        .replace('\\', "/");
      report.push(format!("    {rel}"));
    }
  }

  if report.is_empty() {
    println!("✓ 知识库译文词典：用了 `kt_tracked` / `core_text_tracked` 的页面都挂了词典拉取");
    return true;
  }
  println!(
    "✗ 知识库译文词典：{} 个页面用了知识库译文却没挂词典拉取",
    report.len()
  );
  for line in &report {
    println!("{line}");
  }
  println!(
    "    → 在页面组件体里调 `data::use_knowledge_i18n()`（或改用 KnowledgePage 外壳）；否则英文 / 西语界面直接打开该页时文案停在中文"
  );
  false
}

/// 跑全部守卫：全部 0 违规返回 `Ok`，否则打印证据并 `bail`。
pub fn check(root: &Path) -> Result<()> {
  println!(
    "结构与架构守卫：{} 条规则 + 一文件一组件 + crate 依赖方向 + 知识库译文词典挂载（对应 docs/code-quality.md）",
    RULES.len()
  );
  let mut failed = run_line_rules(root);

  if !check_one_component_per_file(root) {
    failed.push("一文件一组件".to_owned());
  }
  if !check_no_reverse_deps(root) {
    failed.push("crate 依赖方向".to_owned());
  }
  if !check_knowledge_dict_is_loaded(root) {
    failed.push("知识库译文词典挂载".to_owned());
  }

  if !failed.is_empty() {
    bail!("结构与架构守卫未通过：{}", failed.join("、"));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_part_strips_comments() {
    assert_eq!(code_part("  // todo!()"), "  ");
    assert_eq!(code_part("  /* todo!() */"), "");
    assert_eq!(code_part("  *todo!()"), "");
    assert_eq!(code_part("  let x = 1; // todo!()"), "  let x = 1; ");
    assert_eq!(code_part("  let x = 1;"), "  let x = 1;");
  }

  #[test]
  fn view_fn_regex_matches_arrow_returns_only() {
    assert_eq!(VIEW_FN_RE.find_iter("-> impl IntoView").count(), 1);
    assert_eq!(VIEW_FN_RE.find_iter("-> AnyView").count(), 1);
    assert_eq!(VIEW_FN_RE.find_iter("-> IntoView").count(), 1);
    // 没有箭头的普通类型不匹配
    assert_eq!(VIEW_FN_RE.find_iter("let x: IntoView = y;").count(), 0);
  }

  #[test]
  fn knowledge_text_usage_and_dict_loading_are_told_apart() {
    assert!(uses_knowledge_text("{move || kt_tracked(s.name)}"));
    assert!(uses_knowledge_text("crate::data::core_text_tracked(label)"));
    // 只是 `use` 进来，不算在用；定义在 data.rs 里也不是页面。
    assert!(!uses_knowledge_text("use crate::data::kt_tracked;"));
    assert!(loads_knowledge_dict("crate::data::use_knowledge_i18n();"));
    assert!(loads_knowledge_dict(
      "<KnowledgePage title=move || t(\"x\")>"
    ));
    assert!(!loads_knowledge_dict("let x = 1;"));
  }

  #[test]
  fn rules_match_expected_text() {
    assert!(has_sticky_top("class=\"sticky top-[66px]\""));
    assert!(!has_sticky_top("class=\"sticky z-10\""));
    assert!(has_alert("window.alert(\"x\")"));
    assert!(has_alert("alert(\"x\")"));
    assert!(has_alert(
      "let _ = js_sys::eval(\"1\"); web_sys::window().alert(msg)"
    ));
    assert!(!has_alert("let alert = 1;"));
    // 自定义函数名里恰好带 `alert(`:不是浏览器原生弹窗。
    assert!(!has_alert("show_alert(msg)"));
    assert!(!has_alert("fn raise_alert(msg: &str)"));
    assert!(!has_alert("push_alert(msg)"));
    assert!(has_browser_api("use wasm_bindgen::prelude::*;"));
    assert!(has_browser_api("web_sys::window()"));
    assert!(!has_browser_api("let dom_api = 1;"));
    assert!(has_hex_color("color: \"#e11d48\","));
    assert!(has_hex_color("let c = \"#b13e53\";"));
    assert!(!has_hex_color("/// 见 `#e11d48` 的说明"));
    assert!(!has_hex_color("let id = \"q3\";"));
    assert!(has_hex_color("assert_eq!(hex, \"#b13e53ff\");"));
    assert!(!has_hex_color("let n = \"#12345\";"));
    assert!(has_panic_macro("todo!()"));
    assert!(has_panic_macro("unimplemented!()"));
  }
}
