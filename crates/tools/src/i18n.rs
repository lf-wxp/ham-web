//! 界面文案校验：解析 `crates/app/src/i18n.rs` 中的 EN / ES 词典并做一致性检查。
//!
//! 词典以「中文原文 → 译文」的形式维护，因此有两类问题只能靠工具发现：
//!
//! 1. **中文侧改了文案，译文侧没跟上** —— 译文条目会静默失效（页面上该句仍是中文，
//!    且词典里留下一条永远命中不了的死条目）；
//! 2. **占位符数量不匹配** —— `tf()` 按 `{}` 出现顺序替换，译文少了占位符会导致
//!    参数丢失，多了则页面上残留一个裸的 `{}`。
//!
//! 本模块把这两类问题变成可在 CI 里跑的硬检查。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};

/// 单个词条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
  pub key: String,
  pub value: String,
  /// 所在行号（从 1 开始），便于报告定位。
  pub line: usize,
}

/// 解析词典：匹配形如 `("中文", "English"),` 的条目。
///
/// 用正则跨行匹配，兼容 `rustfmt` 把长条目拆成多行的格式：
/// ```text
///   (
///     "中文",
///     "English",
///   ),
/// ```
/// 早期的逐行解析遇到这种格式会把整条漏掉，导致覆盖率统计假性下降。
pub fn parse_entries(src: &str) -> Vec<Entry> {
  // 结尾的 `,?` 很关键：单行词条是 `"world"),`，而 rustfmt 拆成的多行是
  // `"world",\n  ),` —— `"` 与 `)` 之间多了一个逗号，漏掉它会导致多行词条全部匹配失败。
  let re = regex::Regex::new(r#"\(\s*"((?:[^"\\]|\\.)*)"\s*,\s*"((?:[^"\\]|\\.)*)"\s*,?\s*\)"#)
    .expect("valid entry regex");
  let mut out = Vec::new();
  for m in re.captures_iter(src) {
    let start = m.get(0).unwrap().start();
    // 跳过整行注释里的伪词条：`// ("注释", "不该解析")` 也会被正则命中。
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    if src[line_start..start].contains("//") {
      continue;
    }
    let key = unescape_all(&m[1]);
    let value = unescape_all(&m[2]);
    if key.trim().is_empty() {
      continue;
    }
    let line = src[..start].matches('\n').count() + 1;
    out.push(Entry { key, value, line });
  }
  out
}

/// 把正则捕获到的字符串内容里的 Rust 转义（`\\` `\"` `\n`）还原为字面值。
fn unescape_all(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut chars = s.chars();
  while let Some(c) = chars.next() {
    if c == '\\' {
      match chars.next() {
        Some('n') => out.push('\n'),
        Some('r') => out.push('\r'),
        Some('t') => out.push('\t'),
        Some('"') => out.push('"'),
        Some('\\') => out.push('\\'),
        Some(other) => {
          out.push('\\');
          out.push(other);
        }
        None => out.push('\\'),
      }
    } else {
      out.push(c);
    }
  }
  out
}

/// 读取从 `start`（必须是 `"`）开始的字符串字面量，返回内容与结束位置。
fn read_string(s: &str, start: usize) -> Option<(String, usize)> {
  let bytes = s.as_bytes();
  if start >= bytes.len() || bytes[start] != b'"' {
    return None;
  }
  let mut out = String::new();
  let mut i = start + 1;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => return Some((out, i + 1)),
      b'\\' => {
        let (ch, next) = unescape(s, i)?;
        out.push(ch);
        i = next;
      }
      _ => {
        // 逐字符取，保证按字符边界推进（中文多字节安全）
        let rest = &s[i..];
        let ch = rest.chars().next()?;
        out.push(ch);
        i += ch.len_utf8();
      }
    }
  }
  None
}

/// 处理常见转义（`\"` `\\` `\n` `\'` `\u{…}` 之外的简单形式）。
fn unescape(s: &str, i: usize) -> Option<(char, usize)> {
  let bytes = s.as_bytes();
  if i + 1 >= bytes.len() {
    return None;
  }
  match bytes[i + 1] {
    b'"' => Some(('"', i + 2)),
    b'\\' => Some(('\\', i + 2)),
    b'n' => Some(('\n', i + 2)),
    b'r' => Some(('\r', i + 2)),
    b't' => Some(('\t', i + 2)),
    b'\'' => Some(('\'', i + 2)),
    b'0' => Some(('\0', i + 2)),
    _ => None,
  }
}

/// 从 `i18n.rs` 中切出某个 `static X: &[(&str, &str)] = &[ … ];` 的数组体。
///
/// 依赖 rustfmt 的输出约定：数组闭合括号 `];` 独占一行（前面带换行）。若日后改动
/// `i18n.rs` 的格式（如把 `];` 与末条同写一行），需同步调整这里的 `\n];` 匹配。
fn slice_array(src: &str, name: &str) -> Option<String> {
  let marker = format!("static {name}: &[(&str, &str)] = &[");
  let start = src.find(&marker)?;
  let body_start = start + marker.len();
  // 数组以 `];` 结束：取最后一个
  let rest = &src[body_start..];
  let end = rest.find("\n];")?;
  Some(rest[..end].to_owned())
}

/// 校验结果。
#[derive(Debug, Default)]
pub struct Report {
  /// 同一词典内重复出现的中文 key（后写的会覆盖先写的）。
  pub duplicates: Vec<(String, usize)>,
  /// 译文里 `{}` 数量与中文不一致。
  pub placeholder_mismatch: Vec<(String, usize, usize)>,
}

/// 校验一个语言的词典。
pub fn check(entries: &[Entry]) -> Report {
  let mut seen: HashMap<&str, usize> = HashMap::new();
  let mut r = Report::default();
  for e in entries {
    if let Some(prev) = seen.insert(e.key.as_str(), e.line) {
      r.duplicates.push((e.key.clone(), prev));
    }
    let zh = count_placeholders(&e.key);
    let tr = count_placeholders(&e.value);
    if zh != tr {
      r.placeholder_mismatch.push((e.key.clone(), zh, tr));
    }
  }
  r
}

/// 统计 `{}` 出现次数（按非重叠方式，避免 `{{}}` 被数两次）。
fn count_placeholders(s: &str) -> usize {
  let bytes = s.as_bytes();
  let mut n = 0;
  let mut i = 0;
  while i + 1 < bytes.len() {
    if bytes[i] == b'{' && bytes[i + 1] == b'}' {
      n += 1;
      i += 2;
    } else {
      i += 1;
    }
  }
  n
}

/// 语言代码 → `i18n.rs` 里的词典数组名。
///
/// 非法语言直接报错：此前写作 `if lang == "en" { "EN" } else { "ES" }`，`--lang fr`
/// 之类的手误会静默写坏 ES 词典。
fn dict_name(lang: &str) -> Result<&'static str> {
  match lang {
    "en" => Ok("EN"),
    "es" => Ok("ES"),
    other => bail!("不支持的语言：{other}（仅支持 en / es）"),
  }
}

/// 把已填写的模板合并回 `i18n.rs` 的词典数组。
///
/// 直接改 Rust 源文件：新条目追加到数组末尾（保留既有顺序与分区注释，避免大范围 diff）。
/// 已存在的 key 跳过，空 text 也跳过 —— 因此可以对着同一份模板反复增量填写。
pub fn add(root: &Path, lang: &str, batch: &Path) -> Result<usize> {
  #[derive(serde::Deserialize)]
  struct Item {
    zh: String,
    text: String,
  }
  let items: Vec<Item> = crate::fsutil::read_json(batch)?;

  let name = dict_name(lang)?;
  let path = root.join("crates/app/src/i18n.rs");
  let src = std::fs::read_to_string(&path)?;
  let marker = format!("static {name}: &[(&str, &str)] = &[");
  let Some(start) = src.find(&marker) else {
    bail!("在 i18n.rs 中找不到 `static {name}` 词典数组");
  };
  let body_start = start + marker.len();
  let Some(rel_end) = src[body_start..].find("\n];") else {
    bail!("`static {name}` 数组未以 `\\n];` 结束");
  };
  let end = body_start + rel_end;

  // 已有 key，避免重复插入
  let known: HashSet<String> = parse_entries(&src[body_start..end])
    .into_iter()
    .map(|e| e.key)
    .collect();

  let mut buf = String::new();
  let mut n = 0;
  buf.push_str("\n  // —— 批量补齐 ——");
  for it in items {
    if it.text.trim().is_empty() || known.contains(&it.zh) {
      continue;
    }
    buf.push_str(&render_entry(&escape_rust(&it.zh), &escape_rust(&it.text)));
    n += 1;
  }
  if n == 0 {
    return Ok(0);
  }

  let mut out = String::with_capacity(src.len() + buf.len());
  out.push_str(&src[..end]);
  out.push_str(&buf);
  out.push_str(&src[end..]);
  std::fs::write(&path, out)?;
  Ok(n)
}

/// 条目单行渲染的最大显示宽度（含两格缩进与结尾逗号）。
///
/// rustfmt 在 `use_small_heuristics = "Default"` 下会把超过该宽度的词条拆成多行。
/// 这里预先产出同一种布局，否则每次 `add-i18n` 之后 `cargo make fmt-check` 都会报
/// diff（本仓库 4000+ 条目的实际布局反推得到，宽度按 East Asian Wide/Fullwidth
/// 记 2 列，与 rustfmt 的统计口径一致）。
const RUSTFMT_ENTRY_MAX_WIDTH: usize = 65;

/// 按 rustfmt 的布局渲染一条词条（放不下时拆成多行）。返回值自带前导换行。
fn render_entry(zh: &str, text: &str) -> String {
  let single = format!("  (\"{zh}\", \"{text}\"),");
  if display_width(&single) <= RUSTFMT_ENTRY_MAX_WIDTH {
    return format!("\n{single}");
  }
  format!("\n  (\n    \"{zh}\",\n    \"{text}\",\n  ),")
}

/// 字符串的显示宽度：East Asian Wide / Fullwidth 字符按 2 列计（与 rustfmt 一致）。
fn display_width(s: &str) -> usize {
  s.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

/// 是否为 East Asian Wide / Fullwidth 字符（`unicode-width` 归类为 2 列的区间）。
fn is_wide(c: char) -> bool {
  matches!(c as u32,
    0x1100..=0x115F
      | 0x2E80..=0x303E
      | 0x3041..=0x33FF
      | 0x3400..=0x4DBF
      | 0x4E00..=0x9FFF
      | 0xA000..=0xA4CF
      | 0xA960..=0xA97F
      | 0xAC00..=0xD7A3
      | 0xF900..=0xFAFF
      | 0xFE10..=0xFE19
      | 0xFE30..=0xFE6F
      | 0xFF00..=0xFF60
      | 0xFFE0..=0xFFE6
      | 0x1F300..=0x1F64F
      | 0x1F900..=0x1F9FF
      | 0x20000..=0x2FFFD
      | 0x30000..=0x3FFFD
  )
}

/// 转成 Rust 字符串字面量的内容。
///
/// 除 `\` 与 `"` 外还要转义控制字符：译文里若含裸换行，直接写进源码会截断字符串
/// 字面量、把 `i18n.rs` 写坏。
fn escape_rust(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '\\' => out.push_str("\\\\"),
      '"' => out.push_str("\\\""),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      _ => out.push(c),
    }
  }
  out
}

/// 源码里出现、但某语言词典里没有的文案（按出现顺序去重）。
pub fn missing(root: &Path, lang: &str) -> Result<Vec<String>> {
  let name = dict_name(lang)?;
  let src = std::fs::read_to_string(root.join("crates/app/src/i18n.rs"))?;
  let body = slice_array(&src, name);
  let entries = body.as_deref().map(parse_entries).unwrap_or_default();
  let known: HashSet<&str> = entries.iter().map(|e| e.key.as_str()).collect();
  let mut used: Vec<String> = collect_call_sites(&root.join("crates/app/src"))?
    .into_iter()
    .filter(|k| !known.contains(k.as_str()))
    .collect();
  used.sort();
  Ok(used)
}

/// 载入 `i18n.rs` 并校验 EN / ES 两个词典。
///
/// 同时统计「源码里调用了 `t()` 但词典里没有」的文案数量 —— 这是衡量翻译进度的指标：
/// 词典条目数只能说明翻了多少，覆盖率才说明还剩多少。
pub fn run(root: &Path) -> Result<bool> {
  let path = root.join("crates/app/src/i18n.rs");
  let src =
    std::fs::read_to_string(&path).with_context(|| format!("读取 {} 失败", path.display()))?;

  let mut ok = true;
  for (name, label) in [("EN", "英文"), ("ES", "西班牙文")] {
    let Some(body) = slice_array(&src, name) else {
      bail!("在 i18n.rs 中找不到 `static {name}` 词典数组");
    };
    let entries = parse_entries(&body);
    let r = check(&entries);
    println!("{label}（{name}）：{} 条词条", entries.len());
    if !r.duplicates.is_empty() {
      ok = false;
      println!("  ✗ 重复词条 {} 条：", r.duplicates.len());
      for (k, prev) in r.duplicates.iter().take(10) {
        println!("      {k}（先前定义于第 {prev} 行）");
      }
    }
    if !r.placeholder_mismatch.is_empty() {
      ok = false;
      println!("  ✗ 占位符数量不一致 {} 条：", r.placeholder_mismatch.len());
      for (k, zh, tr) in r.placeholder_mismatch.iter().take(10) {
        println!("      {k}：中文 {zh} 个 / 译文 {tr} 个");
      }
    }
    if r.duplicates.is_empty() && r.placeholder_mismatch.is_empty() {
      println!("  ✓ 无重复 / 占位符一致");
    }
  }

  // 覆盖率：源码里出现的中文文案有多少已经有了译文
  let app_src = root.join("crates/app/src");
  match collect_call_sites(&app_src) {
    Ok(used) => {
      let en_body = slice_array(&src, "EN");
      let en_entries = en_body.as_deref().map(parse_entries);
      let en_keys: HashSet<&str> = en_entries
        .as_ref()
        .map(|v| v.iter().map(|e| e.key.as_str()).collect())
        .unwrap_or_default();
      let missing: Vec<&String> = used
        .iter()
        .filter(|k| !en_keys.contains(k.as_str()))
        .collect();
      let done = used.len() - missing.len();
      let pct = if used.is_empty() {
        100.0
      } else {
        done as f64 * 100.0 / used.len() as f64
      };
      println!(
        "\n界面文案覆盖率：{done} / {}（{pct:.1}%）已有英文，缺 {missing_count} 条",
        used.len(),
        missing_count = missing.len()
      );
      for k in missing.iter().take(5) {
        println!("      待补：{k}");
      }
      if missing.len() > 5 {
        println!("      … 另有 {} 条", missing.len() - 5);
      }
    }
    Err(e) => println!("\n（跳过覆盖率统计：{e:#}）"),
  }

  Ok(ok)
}

/// 枚举源码里出现的所有 `t("…")` / `tf("…")` 中文原文，用于统计「已接入但未翻译」。
pub fn collect_call_sites(app_src: &Path) -> Result<HashSet<String>> {
  let mut out = HashSet::new();
  for entry in walk(app_src)? {
    let src = std::fs::read_to_string(&entry)?;
    for caps in simple_caps(&src) {
      out.insert(caps);
    }
  }
  Ok(out)
}

fn walk(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
  let mut out = Vec::new();
  let mut stack = vec![dir.to_path_buf()];
  while let Some(d) = stack.pop() {
    for e in std::fs::read_dir(&d).with_context(|| format!("读取目录 {} 失败", d.display()))?
    {
      let e = e?;
      let p = e.path();
      if p.is_dir() {
        stack.push(p);
      } else if p.extension().is_some_and(|x| x == "rs") {
        out.push(p);
      }
    }
  }
  Ok(out)
}

/// 提取源码中 `t("…")` 的中文参数（只处理不含转义的简单字面量）。
fn simple_caps(src: &str) -> Vec<String> {
  let mut out = Vec::new();
  let bytes = src.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    // 找 t( 或 tf(；要求 `t` 不是更长标识符的结尾，否则 `split("…")` / `insert("…")`
    // 这类以 `t` 结尾的调用会被误当成文案调用，把覆盖率与 `--missing` 结果带偏。
    let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
    if boundary
      && i + 2 < bytes.len()
      && bytes[i] == b't'
      && (bytes[i + 1] == b'(' || (bytes[i + 1] == b'f' && bytes[i + 2] == b'('))
    {
      let open = if bytes[i + 1] == b'(' { i + 2 } else { i + 3 };
      // 注意：解析失败时不能 `continue`，否则跳过末尾的 `i += 1` 导致死循环。
      if let Some(s) = read_string(src, open).map(|(s, _)| s)
        && s.chars().any(|c| ('一'..='鿿').contains(&c))
      {
        out.push(s);
      }
    }
    i += 1;
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_entries_with_commas_and_escapes() {
    let src = r#"
  ("首页", "Home"),
  ("含, 逗号", "With, comma"),
  ("引号 \" 测试", "quote \" test"),
  // ("注释里的", "不该被解析"),
"#;
    let e = parse_entries(src);
    let keys: Vec<_> = e.iter().map(|x| x.key.as_str()).collect();
    assert_eq!(keys, vec!["首页", "含, 逗号", "引号 \" 测试"]);
    assert_eq!(e[0].value, "Home");
  }

  #[test]
  fn detects_duplicate_and_placeholder_mismatch() {
    // 第二条是重复 key（占位符数量正确），第三条才真的少了占位符。
    let src = r#"
  ("共 {} 题", "{} questions"),
  ("共 {} 题", "{} items"),
  ("答对 {} / {}", "{} right"),
"#;
    let r = check(&parse_entries(src));
    assert_eq!(r.duplicates.len(), 1, "{:?}", r.duplicates);
    assert_eq!(
      r.placeholder_mismatch.len(),
      1,
      "{:?}",
      r.placeholder_mismatch
    );
    assert_eq!(r.placeholder_mismatch[0].1, 2); // 中文 2 个
    assert_eq!(r.placeholder_mismatch[0].2, 1); // 译文 1 个
  }

  #[test]
  fn counts_placeholders() {
    assert_eq!(count_placeholders("a {} b {} c"), 2);
    assert_eq!(count_placeholders("无占位符"), 0);
  }

  #[test]
  fn finds_array_body() {
    let src = "static EN: &[(&str, &str)] = &[\n  (\"a\", \"b\"),\n];\nstatic OTHER: u8 = 1;\n";
    let body = slice_array(src, "EN").expect("EN");
    assert!(body.contains("\"a\", \"b\""));
    assert!(!body.contains("OTHER"));
  }

  /// 回归：`t(` 后不是引号时 `read_string` 返回 None，若用 `continue` 会跳过
  /// `i += 1` 导致死循环（此测试一旦回归会直接超时，而非失败）。
  #[test]
  fn simple_caps_survives_malformed_call() {
    let src = "let a = t(1); let b = t(\"正常文案\");";
    let got = simple_caps(src);
    assert_eq!(got, vec!["正常文案"]);
  }

  /// `split("…")` / `insert("…")` 以 `t` 结尾，不能被当成 `t()` 文案调用。
  #[test]
  fn simple_caps_ignores_identifiers_ending_in_t() {
    let src = r#"let a = s.split("天线"); let b = t("正常文案");"#;
    assert_eq!(simple_caps(src), vec!["正常文案"]);
  }

  /// 渲染布局必须与 rustfmt 一致，否则 `cargo make fmt-check` 会在 `add-i18n` 之后报 diff。
  #[test]
  fn renders_entries_in_rustfmt_layout() {
    assert_eq!(render_entry("短", "short"), "\n  (\"短\", \"short\"),");
    let zh = "中".repeat(30);
    assert_eq!(
      render_entry(&zh, "a long translation"),
      format!("\n  (\n    \"{zh}\",\n    \"a long translation\",\n  ),")
    );
    // 中文按 2 列、ASCII 按 1 列。
    assert_eq!(display_width("中a"), 3);
  }

  /// 控制字符必须转义，否则含换行的译文会写坏 `i18n.rs` 的字符串字面量。
  #[test]
  fn escapes_control_chars() {
    assert_eq!(escape_rust("a\nb"), "a\\nb");
    assert_eq!(escape_rust("引号\"与\\"), "引号\\\"与\\\\");
  }
}
