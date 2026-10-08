//! 知识库正文的翻译维护工具链。
//!
//! 与题库解析（`data/explanations.json` + `cargo make explanations-*`）同构：
//! **导出待译模板 → 填写译文 → 合并回写 → 校验覆盖率**。
//!
//! `crates/core` 的 133 个模块里有约 5.2 万个中文字符的知识正文，一次性翻译不现实，
//! 因此按模块推进：每次挑一个模块导出模板，翻完合并，覆盖率会逐步上升。
//!
//! 译文以「中文原文 → 译文」存于 `data/knowledge-i18n/{lang}/{module}.json`，
//! 由 [`crate::fsutil::Paths`] 定位到项目根。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ham_web_core::text::has_cjk;

/// 支持的语言目录名。
pub const LANGS: [&str; 2] = ["en", "es"];

/// 一个模块里待翻译的条目。
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Item {
  /// 中文原文（同时作为 key）。
  pub zh: String,
  /// 译文；空串表示尚未翻译。
  pub text: String,
}

/// 提取 Rust 源码里所有含中文的字符串字面量（按出现顺序，去重保序）。
///
/// 只取字面量，不区分它是常量名还是注释 —— 注释不需要翻译，因此把 `//!` / `//`
/// 开头的行整行跳过；`#[doc]` 与行内注释同理。
pub fn extract_chinese(src: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  let mut seen: HashMap<String, ()> = HashMap::new();
  for line in src.lines() {
    let t = line.trim_start();
    if t.starts_with("//") {
      continue;
    }
    for lit in string_literals(line) {
      // 只收首次出现的字面量（去重保序）：`seen.insert` 返回 Some 表示**已存在**，
      // 判据是 `!contains_key` 而不是 `is_some`。
      // CJK 判定与前端、文案工具共用一份实现（原来只认基本区，会漏掉扩展 A / 兼容区）。
      if has_cjk(&lit) && !seen.contains_key(&lit) {
        seen.insert(lit.clone(), ());
        out.push(lit);
      }
    }
  }
  out
}

/// 去掉行尾行内注释，避免把注释文字当字面量。
///
/// 逐字符扫描以跳过字符串字面量里的 `//`（如 `http://…`），否则 `"http://example.com"`
/// 这类含 `//` 的字符串会被误当作注释起点而截断。
fn strip_trailing_comment(line: &str) -> &str {
  let bytes = line.as_bytes();
  let mut in_string = false;
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => in_string = !in_string,
      b'\\' if in_string => i += 1, // 跳过转义字符，避免 `\"` 被当作字符串结束
      b'/' if !in_string && i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
        return &line[..i];
      }
      _ => {}
    }
    i += 1;
  }
  line
}

/// 取一行里的所有字符串字面量（支持 `\"` 转义）。
fn string_literals(line: &str) -> Vec<String> {
  let code = strip_trailing_comment(line);
  let mut out = Vec::new();
  let bytes = code.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] != b'"' {
      i += 1;
      continue;
    }
    let mut s = String::new();
    i += 1;
    while i < bytes.len() && bytes[i] != b'"' {
      if bytes[i] == b'\\'
        && let Some((ch, next)) = unescape(code, i)
      {
        s.push(ch);
        i = next;
        continue;
      }
      let ch = code[i..].chars().next().unwrap_or('\0');
      s.push(ch);
      i += ch.len_utf8();
    }
    i += 1; // 跳过收尾引号
    out.push(s);
  }
  out
}

/// 解析 `code[i]` 处的反斜杠转义，返回（还原后的字符，下一个字节位置）。
///
/// 必须还原成**运行时字符串**里的字符：`\n` 是换行而不是字母 `n`。否则导出的 key 与
/// 页面里 `kt()` 拿到的文本对不上，译文会永远命中不到，而且没有任何报错。
///
/// 未知转义返回 `None`（调用方把反斜杠当普通字符）：这类源码本身编译不过，跳过比猜一个
/// 字符更安全 —— 而且**绝不能**回退成 `char::from(b)` 再返回 `i + 2`：`b` 可能是某个
/// 多字节字符的首字节，那样下一轮 `code[i..]` 会按非字符边界切片，直接 panic
/// （口径与 `crates/tools/src/i18n.rs` 的同名函数一致）。
fn unescape(code: &str, i: usize) -> Option<(char, usize)> {
  let b = *code.as_bytes().get(i + 1)?;
  match b {
    b'"' => Some(('"', i + 2)),
    b'\\' => Some(('\\', i + 2)),
    b'\'' => Some(('\'', i + 2)),
    b'n' => Some(('\n', i + 2)),
    b'r' => Some(('\r', i + 2)),
    b't' => Some(('\t', i + 2)),
    b'0' => Some(('\0', i + 2)),
    // `\xNN`：恰好两位十六进制。
    b'x' => {
      let v = u8::from_str_radix(code.get(i + 2..i + 4)?, 16).ok()?;
      Some((char::from(v), i + 4))
    }
    // `\u{…}`：1–6 位十六进制。
    b'u' => {
      let rest = code.get(i + 2..)?.strip_prefix('{')?;
      let end = rest.find('}')?;
      let v = u32::from_str_radix(&rest[..end], 16).ok()?;
      Some((char::from_u32(v)?, i + 4 + end))
    }
    // 未知转义：交给调用方按普通字符处理（见上面的说明，不能自己造一个字符）。
    _ => None,
  }
}

/// 校验模块名：只允许 `[a-z0-9_-]`。
///
/// 模块名会被直接拼进路径（`crates/core/src/{module}.rs`、`data/knowledge-i18n/…`），
/// 不校验的话 `--module ../../x` 就能读写项目外的文件。
fn validate_module(module: &str) -> Result<()> {
  if module.is_empty()
    || !module
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
  {
    bail!("模块名非法（只允许小写字母、数字、下划线、连字符）：{module}");
  }
  Ok(())
}

/// 译文文件路径。
fn dict_path(root: &Path, lang: &str, module: &str) -> PathBuf {
  root
    .join("data/knowledge-i18n")
    .join(lang)
    .join(format!("{module}.json"))
}

/// 导出某模块的待译模板；返回写入路径与条目数。
pub fn export(root: &Path, module: &str, lang: &str, out: &Path) -> Result<(PathBuf, usize)> {
  validate_module(module)?;
  let src_path = root.join("crates/core/src").join(format!("{module}.rs"));
  let src = std::fs::read_to_string(&src_path)
    .with_context(|| format!("读取 {} 失败", src_path.display()))?;
  let zhs = extract_chinese(&src);

  // 已有译文直接带上，未翻的留空
  let done = load_dict(root, lang, module)?;
  let items: Vec<Item> = zhs
    .into_iter()
    .map(|zh| Item {
      text: done.get(&zh).cloned().unwrap_or_default(),
      zh,
    })
    .collect();
  let n = items.len();
  if let Some(dir) = out.parent() {
    std::fs::create_dir_all(dir)?;
  }
  crate::fsutil::write_json(out, &items)?;
  Ok((out.to_path_buf(), n))
}

/// 合并已填写的模板回词典。
pub fn add(root: &Path, module: &str, lang: &str, batch: &Path) -> Result<usize> {
  validate_module(module)?;
  let items: Vec<Item> = crate::fsutil::read_json(batch)?;
  let path = dict_path(root, lang, module);
  let mut dict = load_dict(root, lang, module)?;
  let mut n = 0;
  for it in items {
    if it.text.trim().is_empty() {
      continue; // 留空表示暂不翻译，跳过
    }
    dict.insert(it.zh, it.text);
    n += 1;
  }
  if let Some(dir) = path.parent() {
    std::fs::create_dir_all(dir)?;
  }
  crate::fsutil::write_json(&path, &dict)?;
  Ok(n)
}

/// 读取某模块某语言的词典。
///
/// 用 `BTreeMap` 而不是 `HashMap`：词典要原样写回仓库，`HashMap` 的迭代顺序随进程
/// 随机（`RandomState`），每次回写都会产出整排序的伪 diff。
pub fn load_dict(root: &Path, lang: &str, module: &str) -> Result<BTreeMap<String, String>> {
  let path = dict_path(root, lang, module);
  if !path.exists() {
    return Ok(BTreeMap::new());
  }
  crate::fsutil::read_json(&path)
}

/// 统计覆盖率：某模块有多少中文条目已有译文。
pub fn coverage(root: &Path, module: &str, lang: &str) -> Result<(usize, usize)> {
  validate_module(module)?;
  let src_path = root.join("crates/core/src").join(format!("{module}.rs"));
  let src = std::fs::read_to_string(&src_path)
    .with_context(|| format!("读取 {} 失败", src_path.display()))?;
  let zhs = extract_chinese(&src);
  let dict = load_dict(root, lang, module)?;
  let done = zhs.iter().filter(|z| dict.contains_key(*z)).count();
  Ok((done, zhs.len()))
}

/// 译文里已经对不上源码的条目（key 漂移）。
///
/// 译文以**中文原文**为 key，源码改了句子就永远命中不到 —— 运行时静默回退中文，
/// 页面上照旧显示，而且没有任何报错（`WSPR_NOTES` 首条就是这么悄悄失效的：
/// 中文改成「mW 级（0 dBm = 1mW）」后，旧 key「5mW」的译文再也用不上）。
/// 因此必须单独校验：词典里的每个 key 都仍要能在源码的中文集合里找到。
pub fn drift(root: &Path) -> Result<Vec<String>> {
  let base = root.join("data/knowledge-i18n");
  let mut out = Vec::new();
  for lang in LANGS {
    let dir = base.join(lang);
    if !dir.exists() {
      continue;
    }
    for e in std::fs::read_dir(&dir).with_context(|| format!("读取 {} 失败", dir.display()))? {
      let path = e?.path();
      // 白名单：无扩展名的文件（`extension()` 为 `None`）也要跳过 —— 以前写的是
      // `is_some_and(|x| x != "json")`，`None` 时整个条件为假、不会 continue，
      // 于是无扩展名文件被当成词典处理（`file_stem()` 返回整个文件名）。
      if path.extension().is_none_or(|x| x != "json") {
        continue;
      }
      let Some(module) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
        continue;
      };
      let src_path = root.join("crates/core/src").join(format!("{module}.rs"));
      if !src_path.exists() {
        out.push(format!("{lang}/{module}：源码模块已不存在（{module}.rs）"));
        continue;
      }
      let src = std::fs::read_to_string(&src_path)
        .with_context(|| format!("读取 {} 失败", src_path.display()))?;
      let zhs: HashSet<String> = extract_chinese(&src).into_iter().collect();
      for key in load_dict(root, lang, &module)?.keys() {
        if !zhs.contains(key) {
          out.push(format!(
            "{lang}/{module}：源码里已没有这句中文（译文命中不到，页面上仍是中文）→ {key}"
          ));
        }
      }
    }
  }
  out.sort();
  Ok(out)
}

/// 列出 core 中含中文的模块及其字符数（从大到小），供决定先翻哪个。
pub fn inventory(root: &Path) -> Result<Vec<(String, usize)>> {
  let dir = root.join("crates/core/src");
  let mut out = Vec::new();
  for e in std::fs::read_dir(&dir).with_context(|| format!("读取 {} 失败", dir.display()))? {
    let p = e?.path();
    if p.extension().is_some_and(|x| x == "rs") {
      let src = std::fs::read_to_string(&p)?;
      let n: usize = extract_chinese(&src)
        .iter()
        .map(|s| s.chars().count())
        .sum();
      if n > 0 {
        let stem = p
          .file_stem()
          .unwrap_or_default()
          .to_string_lossy()
          .to_string();
        out.push((stem, n));
      }
    }
  }
  out.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_chinese_literals_in_order() {
    let src = r#"
//! 模块注释不该被提取
pub const A: &[(&str, &str)] = &[("信标", "周期发送呼号"), ("传播研究", "汇总数据")];
pub const B: &[&str] = &["发射功率可低至 5mW。"];
"#;
    let got = extract_chinese(src);
    assert_eq!(
      got,
      vec![
        "信标",
        "周期发送呼号",
        "传播研究",
        "汇总数据",
        "发射功率可低至 5mW。"
      ]
    );
  }

  #[test]
  fn skips_comments_and_dedupes() {
    let src = r#"
// 这是注释，不该被提取
pub const A: &str = "重复";
pub const B: &str = "重复";
pub const C: &str = "另一个";
"#;
    let got = extract_chinese(src);
    assert_eq!(got, vec!["重复", "另一个"]);
  }

  #[test]
  fn handles_escaped_quotes() {
    let src = r#"pub const A: &str = "他说「你好」";  // 行内注释
"#;
    let got = extract_chinese(src);
    assert!(got.iter().any(|s| s.contains("你好")), "{got:?}");
    assert!(!got.iter().any(|s| s.contains("行内注释")), "{got:?}");
  }

  /// 转义必须还原成**运行时字符串**里的字符：`\n` 是换行而非字母 `n`。
  /// 否则导出的 key 与页面里 `kt()` 拿到的文本对不上，译文会静默失效。
  /// 未知转义（如 `\` 后紧跟多字节字符）**不能 panic**：以前未知分支返回 `i + 2`，
  /// 而那是多字节字符的中间字节，下一轮 `code[i..]` 按非字符边界切片会直接炸
  /// （报错信息还与真实原因无关）。这类源码本身编译不过，但工具扫描的是任意 `.rs` 文本。
  #[test]
  fn unknown_escapes_do_not_panic_on_multibyte() {
    let src = "pub const C: &str = \"前缀\\中 文\";";
    let got = extract_chinese(src);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0], "前缀\\中 文");
  }

  #[test]
  fn restores_escapes_like_runtime_strings() {
    let src = "pub const A: &str = \"第一行\\n第二行\";";
    assert_eq!(extract_chinese(src), vec!["第一行\n第二行"]);
    // `\u{…}` 十六进制转义同样按实际字符还原。
    let src2 = "pub const B: &str = \"\\u{5929}\\u{7ebf}\";";
    assert_eq!(extract_chinese(src2), vec!["天线"]);
  }
}
