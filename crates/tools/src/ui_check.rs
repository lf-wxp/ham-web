//! 界面规范的机械守卫：把 `docs/ui-components.md` §11 里「应为 0」的几条搬进 `cargo make check`。
//!
//! 为什么落成工具：`rounded-*` 这类「写了也没效果」的类曾被 `surfaces.css` 当存量卡片皮肤的
//! 钩子用（2026-10-13 才换成 `pxl-card` 标记清理干净），手写 `<input>` 与 `input_class()` 类名
//! 工厂也都是收口之后容易回潮的东西 —— 靠人记得手跑 §11 的命令不可靠。
//!
//! 覆盖 §11 的 ①②③④⑤⑦，以及 ⑥ 与 ⑧ 的机械部分：
//! - ①②③④⑤⑦ 命中即失败；
//! - ⑥ 的任意值阴影只允许「第三个长度（模糊半径）为 0」的像素硬阴影（模糊位写成 `var(…)`
//!   判不出来时放过，宁可漏报）；
//! - ⑧ 的页面级宽度容器是**封闭清单** —— 清单外命中、清单内计数漂移都算失败。
//!
//! 判不了、仍留给人工的：`mm` 尺寸的打印页（⑧ 的例外）、内联 `style` 里的阴影、
//! ⑨ 的例外注释计数。
//!
//! 判定刻意保守：
//! - 每行先剥掉 `//` 之后的内容，`/*` 与 `*` 开头的行整行跳过（历史说明不算违规）；
//!   字符串里含 `//`（URL）的行会被截断，只可能漏报、不会误报。
//! - `cn.rs`（类名合并器，字符串里出现工具类名是它的本职工作）按文件名跳过。

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// 每条规则最多打印多少条命中，避免一次刷屏。
const MAX_REPORT: usize = 10;

/// 一条守卫规则。
struct Rule {
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

const RULES: &[Rule] = &[
  Rule {
    name: "已失效的圆角类（`rounded-*`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_rounded_class,
    advice: "卡片皮肤用 `pxl-card` 标记（§2「可用像素元素」），其余圆角一律不要写",
  },
  Rule {
    name: "本地控件样式常量（`const INPUT:` / `const TEXTAREA:`）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("const INPUT:") || line.contains("const TEXTAREA:"),
    advice: "控件样式只在 ui/ 里定义；页面要局部调整就用组件的 `class` 参数",
  },
  Rule {
    name: "原生表单控件（`<input>` / `<select>` / `<textarea>`）",
    roots: &["crates/app/src/pages", "crates/app/src/components"],
    skip: &[],
    hits: has_native_control,
    advice: "用 ui/ 的 Input / NativeSelect / Textarea / FileInput / FileDropZone（§3 决策表）",
  },
  Rule {
    name: "类名工厂 `input_class(`（已删除）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("input_class("),
    advice: "表单控件已全部走 ui/，不要再造「让未迁移输入框长得一样」的类名工厂",
  },
  Rule {
    name: "手写 chip 样式常量（`CHIP_ON` 等）",
    roots: &["crates/app/src"],
    skip: &["/ui/"],
    hits: |line| line.contains("CHIP_ON"),
    advice: "用 ui/ 的 ChipGroup / Chip / ChipToggle",
  },
  Rule {
    name: "把填充色 token 当文字色（对比度不够）",
    roots: &["crates/app/src"],
    skip: &[],
    hits: has_fill_token_as_text,
    advice: "文字用 `--pxl-ink` / 语义前景色，填充色只做背景（§9）",
  },
  Rule {
    name: "模糊 / 渐变（零模糊零渐变）",
    roots: &["crates/app/src"],
    skip: &["/ui/", "cn.rs"],
    hits: has_blur_or_gradient,
    advice: "去掉 `backdrop-blur` / `blur-*` / `drop-shadow-*` / `bg-gradient-*` / `bg-linear-*`（§2 硬规则 1）",
  },
  Rule {
    name: "带模糊半径的任意值阴影（`shadow-[…]`）",
    roots: &["crates/app/src"],
    skip: &["cn.rs"],
    hits: has_soft_arbitrary_shadow,
    advice: "任意值阴影只写像素硬阴影 `x y 0 0 <色>`（模糊位必须是 0）；要层次感就加粗描边 / 换色，命名档位（`shadow-sm` 等）在 tokens.css 里已经是硬阴影、可以直接用",
  },
];

/// 一行里有没有圆角**类**：只看字符串字面量内部的 `rounded` token。
///
/// 必须限定在字面量里 —— `let rounded: f64` / `rounded.abs()` 这种标识符（`dbm_dbuv.rs`）
/// 不是类名，按整行匹配会误报。
fn has_rounded_class(line: &str) -> bool {
  has_rounded_token(&string_contents(line))
}

/// 收集一行里所有字符串字面量的内容（`\"` / `\\` 转义按字面处理），字面量之间补空格，
/// 避免相邻两个字面量拼成一个 token 而误判。未闭合的字面量按「到行尾」处理。
fn string_contents(line: &str) -> String {
  let mut out = String::new();
  let mut rest = line;
  while let Some(start) = rest.find('"') {
    let after = &rest[start + 1..];
    let mut escaped = false;
    let mut end = None;
    for (i, c) in after.char_indices() {
      if escaped {
        escaped = false;
      } else if c == '\\' {
        escaped = true;
      } else if c == '"' {
        end = Some(i);
        break;
      }
    }
    match end {
      Some(i) => {
        out.push_str(&after[..i]);
        rest = &after[i + 1..];
      }
      None => {
        out.push_str(after);
        break;
      }
    }
    out.push(' ');
  }
  out
}

/// `rounded` / `rounded-*` 的 token 判定（在「类名文本」上跑，不含引号）。
///
/// 前后做标识符边界：`roundedness`、`x_rounded_y` 不算；变体前缀（`focus:rounded-md`）算 ——
/// 它同样是被全局零圆角规则压平的类。
fn has_rounded_token(line: &str) -> bool {
  const NEEDLE: &str = "rounded";
  let bytes = line.as_bytes();
  let mut from = 0;
  while let Some(pos) = line[from..].find(NEEDLE) {
    let start = from + pos;
    let end = start + NEEDLE.len();
    let boundary = |c: u8| !c.is_ascii_alphanumeric() && c != b'_';
    let prev_ok = start == 0 || boundary(bytes[start - 1]);
    let next_ok = end == bytes.len() || boundary(bytes[end]);
    if prev_ok && next_ok {
      return true;
    }
    from = end;
  }
  false
}

fn has_native_control(line: &str) -> bool {
  line.contains("<input") || line.contains("<select") || line.contains("<textarea")
}

/// §11 ⑦：把填充色 token（hp / xp / gold / win）当文字色用，对比度不达标。
fn has_fill_token_as_text(line: &str) -> bool {
  ["hp", "xp", "gold", "win"]
    .iter()
    .any(|t| line.contains(&format!("text-[color:var(--pxl-{t})]")))
}

/// §11 ⑥：模糊 / 渐变。
///
/// 命名档位（`blur-sm` … `blur-3xl`）、任意值（`blur-[…]`）与 `drop-shadow-*` 都算 ——
/// 像素规范里只能靠描边 / 换色做层次；内联 `style="filter: drop-shadow(0 1px 0 …)"`
/// （`bubble.rs` 的对话气泡箭头，模糊 0）不在这条规则的范围内。
fn has_blur_or_gradient(line: &str) -> bool {
  const NEEDLES: &[&str] = &[
    "backdrop-blur",
    "blur-sm",
    "blur-md",
    "blur-lg",
    "blur-xl",
    "blur-2xl",
    "blur-3xl",
    "blur-[",
    "drop-shadow-",
    "bg-gradient",
    "bg-linear",
  ];
  NEEDLES.iter().any(|n| line.contains(n))
}

/// §11 ⑥：任意值阴影 `shadow-[…]` 里有没有模糊半径。
///
/// 命名档位不用判 —— `tokens.css` 把 `--shadow-*` 重定义成了 `x y 0 0 色` 的硬阴影，
/// `shadow-sm` / `shadow-lg` 本身就是合规的像素硬阴影；要拦的是用任意值绕过 token 系统
/// 写出来的软阴影（`shadow-[0_4px_12px_…]`）。
fn has_soft_arbitrary_shadow(line: &str) -> bool {
  arbitrary_values(line, "shadow-[").iter().any(|value| {
    split_shadow_list(value)
      .iter()
      .any(|seg| shadow_segment_has_blur(seg))
  })
}

/// 取出一行里所有 `<prefix>[…]` 的方括号内容（`shadow-[…]`、`blur-[…]` 这类）。
fn arbitrary_values(line: &str, prefix: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut from = 0usize;
  while let Some(pos) = line[from..].find(prefix) {
    let start = from + pos + prefix.len();
    let mut depth = 1i32;
    let mut end = None;
    for (i, c) in line[start..].char_indices() {
      match c {
        '[' => depth += 1,
        ']' => {
          depth -= 1;
          if depth == 0 {
            end = Some(i);
            break;
          }
        }
        _ => {}
      }
    }
    match end {
      Some(i) => {
        out.push(line[start..start + i].to_owned());
        from = start + i + 1;
      }
      None => break,
    }
  }
  out
}

/// 按顶层逗号切分多层阴影（`a, b`），`rgb(0,0,0,.2)` 里的逗号不算。
fn split_shadow_list(value: &str) -> Vec<&str> {
  let mut out = Vec::new();
  let mut depth = 0i32;
  let mut start = 0usize;
  for (i, c) in value.char_indices() {
    match c {
      '(' => depth += 1,
      ')' => depth -= 1,
      ',' if depth == 0 => {
        out.push(&value[start..i]);
        start = i + 1;
      }
      _ => {}
    }
  }
  out.push(&value[start..]);
  out
}

/// 单个阴影段（`0_2px_0_0_var(--pxl-shadow)`）的模糊位（第 3 个长度）是不是**明确的非 0 长度**。
///
/// - 长度位不足 3 个（`x y`、`x y 色`）：模糊半径按 CSS 默认取 0 → 合规；
/// - 模糊位写成 `var(…)` / `calc(…)`（如 `0_2px_var(--pxl-shadow)`，那其实是颜色）：
///   静态判不出来 → **放过**（宁可漏报也不拦人的合法写法）；
/// - 明确写死非 0（`0_0_10px_#000`）：拦。
fn shadow_segment_has_blur(segment: &str) -> bool {
  let mut tokens = segment.split('_').filter(|t| !t.is_empty());
  // `inset` 只是关键字，跳过它再数长度
  while tokens.clone().next().is_some_and(|t| t == "inset") {
    tokens.next();
  }
  let mut lengths: Vec<&str> = Vec::new();
  for token in tokens {
    if is_shadow_length(token) {
      lengths.push(token);
    } else {
      break; // 到颜色 / 其它关键字为止
    }
  }
  lengths.get(2).is_some_and(|blur| {
    !blur.starts_with("var(") && !blur.starts_with("calc(") && *blur != "0" && *blur != "0px"
  })
}

/// CSS 长度：`0`、`2px`、`-2px`、`.5rem`、`50%`、`var(…)`、`calc(…)`。
fn is_shadow_length(token: &str) -> bool {
  let t = token.strip_prefix('-').unwrap_or(token);
  if t == "0" || t.starts_with("var(") || t.starts_with("calc(") {
    return true;
  }
  let mut chars = t.chars();
  chars.next().is_some_and(|c| c.is_ascii_digit() || c == '.')
    && t
      .chars()
      .all(|c| c.is_ascii_digit() || c == '.' || c == '%' || c.is_ascii_alphabetic())
}

/// §4 的「页面级宽度容器」封闭清单：允许出现这类宽度的文件 + 各自的说明。
///
/// 键 = 相对项目根的路径。**新增一处就得同时改这里与 §4 的清单** —— 这正是「封闭清单」的意思。
const WIDTH_ALLOWED: &[(&str, &str)] = &[
  ("crates/app/src/app/footer.rs", "页脚"),
  ("crates/app/src/app/storage_warning.rs", "存储告警横幅"),
  (
    "crates/app/src/components/common/bottom_bar.rs",
    "底部工具栏对齐行",
  ),
  (
    "crates/app/src/components/common/page_header.rs",
    "页头对齐行（多行页头）",
  ),
  (
    "crates/app/src/components/navigation/nav_bar.rs",
    "顶栏 container",
  ),
  (
    "crates/app/src/pages/bookmarks/bookmarks_page.rs",
    "多行页头工具条（§4 表里登记的对齐行）",
  ),
  (
    "crates/app/src/pages/mistakes/mistakes_page.rs",
    "多行页头工具条（§4 表里登记的对齐行）",
  ),
  (
    "crates/app/src/pages/qsl_labels/qsl_labels_page.rs",
    "打印说明行",
  ),
];

/// 清单内的命中数 / 文件数（复核日期 2026-10-13）。漂移了就要人看一眼。
const WIDTH_EXPECT_HITS: usize = 10;
const WIDTH_EXPECT_FILES: usize = 8;

/// §11 ⑧：页面里写页面级宽度容器（`max-w-2xl…6xl`，或类名开头的 `container`）。
///
/// 与手工命令同口径：只认 `class="…"` 字面量；含 `min-w-` / `sm:max-w` 的行、`<Dialog` 的行放过
/// （`dialog` 宽度是控件自身的，不是内容宽度）。`max-w-[210mm]` 这类打印纸尺寸也不在范围内。
fn has_page_width_container(line: &str) -> bool {
  if line.contains("min-w-") || line.contains("sm:max-w") || line.contains("<Dialog") {
    return false;
  }
  const WIDTHS: &[&str] = &[
    "max-w-2xl",
    "max-w-3xl",
    "max-w-4xl",
    "max-w-5xl",
    "max-w-6xl",
  ];
  class_literal_values(line).iter().any(|value| {
    WIDTHS.iter().any(|w| value.contains(w))
      || value
        .split_whitespace()
        .next()
        .is_some_and(|t| t == "container")
  })
}

/// 一行里所有 `class="…"` 字面量的取值。
fn class_literal_values(line: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut from = 0usize;
  while let Some(pos) = line[from..].find("class=\"") {
    let start = from + pos + "class=\"".len();
    let end = line[start..].find('"').map_or(line.len(), |i| start + i);
    out.push(line[start..end].to_owned());
    from = end;
  }
  out
}

/// 剥掉行注释，得到「参与判定的代码部分」。
///
/// `/*` 与 `*` 开头的整行按块注释续行处理（`*slot = …` 这种解引用语句因此不参与扫描，
/// 代价是漏报 —— 它本来也不该出现类名字符串）。
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

/// §11 ⑧ 的封闭清单：清单外的命中要修；清单内的计数漂移要人确认（同时改 §4 与本表）。
fn check_width_containers(root: &Path) -> bool {
  let mut inside_hits = 0usize;
  let mut inside_files: BTreeSet<String> = BTreeSet::new();
  let mut outside_hits = 0usize;
  let mut report: Vec<String> = Vec::new();

  for dir in [
    "crates/app/src/pages",
    "crates/app/src/components",
    "crates/app/src/app",
  ] {
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
      let Ok(text) = std::fs::read_to_string(path) else {
        continue;
      };
      for (i, line) in text.lines().enumerate() {
        let code = code_part(line);
        if code.is_empty() || !has_page_width_container(code) {
          continue;
        }
        if WIDTH_ALLOWED.iter().any(|(p, _)| *p == rel) {
          inside_hits += 1;
          inside_files.insert(rel.clone());
        } else {
          outside_hits += 1;
          if report.len() < MAX_REPORT {
            report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
          }
        }
      }
    }
  }

  let drift = (inside_hits, inside_files.len()) != (WIDTH_EXPECT_HITS, WIDTH_EXPECT_FILES);
  if outside_hits == 0 && !drift {
    println!(
      "✓ 页面级宽度容器（`max-w-2xl…6xl` / `container`）：清单内 {inside_hits} 处 / {} 文件（= §4）",
      inside_files.len()
    );
    return true;
  }
  if drift {
    println!(
      "✗ 页面级宽度容器：清单内计数变了（{} 处 / {} 文件，期望 {} 处 / {} 文件）",
      inside_hits,
      inside_files.len(),
      WIDTH_EXPECT_HITS,
      WIDTH_EXPECT_FILES
    );
    let list = WIDTH_ALLOWED
      .iter()
      .map(|(p, what)| format!("{p}（{what}）"))
      .collect::<Vec<_>>()
      .join("、");
    println!("    清单：{list}");
  }
  if outside_hits > 0 {
    println!("✗ 页面级宽度容器：清单外 {outside_hits} 处");
    for line in &report {
      println!("{line}");
    }
    println!(
      "    → 内容页只有一种宽度（§4）：用 `PageHeader` + `PageContainer`；确实要新增对齐行，就在 §4 清单与本工具的白名单里同时登记"
    );
  }
  false
}

/// 跑全部规则：全部 0 违规返回 `Ok`，否则打印证据并 `bail`。
pub fn check(root: &Path) -> Result<()> {
  println!(
    "界面规范守卫：{} 条规则 + 1 份封闭清单（对应 docs/ui-components.md §11）",
    RULES.len()
  );
  let mut failed: Vec<String> = Vec::new();

  for rule in RULES {
    let mut found = 0usize;
    let mut report: Vec<String> = Vec::new();
    for dir in rule.roots {
      let base = root.join(dir);
      for entry in WalkDir::new(&base)
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
        if rule.skip.iter().any(|s| rel.contains(s)) {
          continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
          continue;
        };
        for (i, line) in text.lines().enumerate() {
          let code = code_part(line);
          if code.is_empty() || !(rule.hits)(code) {
            continue;
          }
          found += 1;
          if report.len() < MAX_REPORT {
            report.push(format!("    {rel}:{}  {}", i + 1, code.trim()));
          }
        }
      }
    }

    if found == 0 {
      println!("✓ {}", rule.name);
    } else {
      println!("✗ {}：{found} 处", rule.name);
      for line in &report {
        println!("{line}");
      }
      if found > report.len() {
        println!("    …（另有 {} 处，先修上面这些）", found - report.len());
      }
      println!("    → {}", rule.advice);
      failed.push(format!("{}（{found} 处）", rule.name));
    }
  }

  if !check_width_containers(root) {
    failed.push("页面级宽度容器（§11 ⑧）".to_owned());
  }

  if !failed.is_empty() {
    bail!("界面规范守卫未通过：{}", failed.join("、"));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rounded_class_only_matches_inside_string_literals() {
    // 标识符（dbm_dbuv.rs 的局部变量）不是类名
    assert!(!has_rounded_class(
      "  let rounded: f64 = format!(\"{v:.5e}\").parse().unwrap_or(v);"
    ));
    assert!(!has_rounded_class(
      "  if rounded.abs() < 1e-5 { 0.0 } else { rounded }"
    ));
    // 类是写在字面量里的；`\"` 转义（HTML 字符串）仍算同一个字面量
    assert!(has_rounded_class("  class=\"px-2 rounded-xl border\""));
    assert!(has_rounded_class(
      "    \"<mark class=\\\"rounded-sm bg-primary/20\\\">\""
    ));
    assert!(has_rounded_class(
      "  let x = format!(\"focus:rounded-md {}\", extra);"
    ));
  }

  #[test]
  fn string_contents_skips_escapes_and_keeps_separate_literals_apart() {
    assert_eq!(string_contents("a \"b c\" d"), "b c ");
    assert_eq!(string_contents("\"a\\\"b\""), "a\\\"b ");
    assert_eq!(string_contents("no literals here"), "");
  }

  #[test]
  fn rounded_token_needs_identifier_boundaries() {
    assert!(has_rounded_token("class=\"px-2 rounded-xl border\""));
    assert!(has_rounded_token("focus:rounded-md focus:bg-primary"));
    assert!(has_rounded_token("\"rounded\""));
    assert!(!has_rounded_token("let roundedness = 1;"));
    assert!(!has_rounded_token("let x_rounded_y = 1;"));
    assert!(!has_rounded_token("class=\"pxl-card border bg-card\""));
  }

  #[test]
  fn comments_are_not_scanned() {
    assert_eq!(code_part("  // class=\"rounded-xl\""), "  ");
    assert_eq!(code_part("  /// 原先写的是 `rounded-full`"), "  ");
    assert_eq!(code_part("  /* rounded-xl */"), "");
    assert_eq!(code_part("  *slot = -*slot;"), "");
    assert_eq!(
      code_part("  class=\"rounded-xl\" // 说明"),
      "  class=\"rounded-xl\" "
    );
  }

  #[test]
  fn other_rules_match_expected_text() {
    assert!(has_native_control("    <input type=\"file\" />"));
    assert!(!has_native_control("    <FileInput on_files=cb />"));
    assert!(has_fill_token_as_text(
      "class=\"text-[color:var(--pxl-gold)]\""
    ));
    assert!(!has_fill_token_as_text(
      "class=\"text-[color:var(--pxl-ink)]\""
    ));
    assert!(has_blur_or_gradient("class=\"backdrop-blur-sm\""));
    assert!(has_blur_or_gradient("class=\"blur-lg\""));
    assert!(has_blur_or_gradient("class=\"drop-shadow-md\""));
    // 内联 `filter: drop-shadow(…)` 不是工具类，模糊 0 的那处（bubble.rs）也不算
    assert!(!has_blur_or_gradient(
      "style=\"filter: drop-shadow(0 1px 0 rgba(0,0,0,.06));\""
    ));
  }

  #[test]
  fn soft_arbitrary_shadow_sees_the_blur_slot() {
    // 仓库里现有的三处（页头 / 移动菜单 / 底部栏）都是像素硬阴影
    assert!(!has_soft_arbitrary_shadow(
      "class=\"border-b-2 bg-card shadow-[0_2px_0_0_var(--pxl-shadow)]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[0_-2px_0_0_var(--pxl-shadow)]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[inset_2px_2px_0_0_#000]\""
    ));
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-[0_2px_var(--pxl-shadow)]\""
    ));
    // 命名档位由 tokens.css 定义成硬阴影，不归这条规则管
    assert!(!has_soft_arbitrary_shadow(
      "class=\"shadow-lg shadow-primary/25\""
    ));
    // 带模糊半径的要拦
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_4px_12px_rgba(0,0,0,.4)]\""
    ));
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_0_10px_#000]\""
    ));
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[inset_0_2px_4px_#000]\""
    ));
    // 多层阴影：任一段有模糊就算
    assert!(has_soft_arbitrary_shadow(
      "class=\"shadow-[0_2px_0_0_#000,0_0_12px_rgba(0,0,0,.5)]\""
    ));
  }

  #[test]
  fn page_width_container_matches_the_documented_shape() {
    assert!(has_page_width_container(
      "  <div class=\"mx-auto flex max-w-5xl flex-wrap items-center gap-2 px-4 pb-3\">"
    ));
    assert!(has_page_width_container(
      "  <div class=\"container relative mx-auto px-1\">"
    ));
    // §4 的例外与排除项
    assert!(!has_page_width_container(
      "  <div class=\"mx-auto min-w-0 max-w-5xl px-4\">"
    ));
    assert!(!has_page_width_container(
      "  <Dialog class=\"max-w-5xl sm:max-w-lg px-4\">"
    ));
    assert!(!has_page_width_container(
      "  <div class=\"print-sheet mx-auto mt-6 max-w-[210mm] bg-white\">"
    ));
    // 用变量 / 组件传宽度的不在这条规则里（PageContainer 自己就是 `class=cn(&[…])`）
    assert!(!has_page_width_container(
      "    <div class=cn(&[\"mx-auto max-w-5xl px-4\", &class])>"
    ));
  }
}
