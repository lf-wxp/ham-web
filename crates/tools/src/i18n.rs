//! 界面文案的校验与回填工具链。
//!
//! 词条的唯一事实源是 `data/i18n/{lang}/{domain}.json`，key 是语义化的
//! `<domain>.<slug>`（中文与其它语言同等对待，只是 `lang = zh` 的一份译文）；
//! `crates/app/build.rs` 在构建期把它们编成静态表。这里**不再解析 Rust 源码里的
//! 词典数组**，只做资源目录的读写与一致性检查 —— 词典结构调整不会再牵动工具。
//!
//! 与题库解析（`data/explanations.json`）同构：**导出待译模板 → 填写 → 合并回写 →
//! 校验覆盖率**。详见 `docs/i18n-refactor.md`。
//!
//! 因为源码里写的就是 key，`check-i18n` 能给出比过去强得多的保证：
//!
//! 1. **源码里用到的 key 必须存在于词典** —— 写错 key 在 CI 就会红，不会等到页面上
//!    漏出一句 `shell.hme`；
//! 2. **词典里的 key 必须有人用** —— 文案删掉后残留的译文能被检出来清理；
//! 3. **占位符数量以中文侧为准** —— `tf()` 按 `{}` 顺序替换，少一个就丢参数。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

// CJK 判定与前端（`catalog` 的中文反向索引）共用一份实现，见函数文档。
use ham_web_core::text::has_cjk;

use anyhow::{Context, Result, bail};

/// 支持的语言目录名。
pub const LANGS: [&str; 3] = ["zh", "en", "es"];

/// 词条值：一句话文案，或按 CLDR 复数类别分的变体。
///
/// 用 `untagged` 让「字符串」与「对象」两种写法在 JSON 里自然共存 —— 作者不必为
/// 不需要复数的文案多套一层对象。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Value {
  /// 普通文案。
  Plain(String),
  /// 复数词条：`类别 → 文案`，必须含 `other`（兜底类别）。
  Plural(BTreeMap<String, String>),
}

impl Value {
  /// 兜底文本：普通文案取自身，复数词条取 `other` 变体。
  #[must_use]
  pub fn text(&self) -> &str {
    match self {
      Self::Plain(s) => s.as_str(),
      Self::Plural(m) => m.get("other").map_or("", |s| s.as_str()),
    }
  }

  /// 待填模板里「还没写」的判断：普通文案看自身是否为空，复数变体看有没有非空变体。
  #[must_use]
  pub fn is_blank(&self) -> bool {
    match self {
      Self::Plain(s) => s.trim().is_empty(),
      Self::Plural(m) => m.values().all(|v| v.trim().is_empty()),
    }
  }
}

/// 域文件（`{key: 值}`）的内存表示：保序，见 [`load_entries`]。
///
/// 值类型只能是 `serde_json::Value` —— `serde_json::Map` 只为它实现了
/// `Serialize` / `Deserialize`；取用时再转成 [`Value`]。
pub type EntryMap = serde_json::Map<String, serde_json::Value>;

/// 单个词条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
  pub key: String,
  pub value: Value,
  /// 所在域（`data/i18n/{lang}/{domain}.json` 的文件名，也是 key 的前缀）。
  pub domain: String,
}

/// 资源目录：`data/i18n/{lang}/`。
fn lang_dir(root: &Path, lang: &str) -> PathBuf {
  root.join("data").join("i18n").join(lang)
}

/// 校验语种；非法语言直接报错，避免 `--lang fr` 之类的手误静默写坏别的语言。
pub fn check_lang(lang: &str) -> Result<&'static str> {
  match lang {
    "zh" => Ok("zh"),
    "en" => Ok("en"),
    "es" => Ok("es"),
    other => bail!("不支持的语言：{other}（仅支持 zh / en / es）"),
  }
}

/// 读入一种语言的全部域文件（按文件名排序，域内保持文件里的 key 顺序）。
pub fn load_entries(root: &Path, lang: &str) -> Result<Vec<Entry>> {
  let dir = lang_dir(root, check_lang(lang)?);
  let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
    .with_context(|| format!("读取 {} 失败", dir.display()))?
    .map(|e| e.context("读取目录项失败").map(|e| e.path()))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .filter(|p| p.extension().is_some_and(|x| x == "json"))
    .collect();
  files.sort();

  let mut out = Vec::new();
  for f in files {
    let domain = f
      .file_stem()
      .and_then(|s| s.to_str())
      .context("域文件名不是合法 UTF-8")?
      .to_owned();
    // 保序（`serde_json` 开了 `preserve_order`）：回填时只该动改到的那几条，
    // 排序会让整个文件的键重排，diff 里全是噪声。输出侧（`pack`）另行排序。
    let map: EntryMap =
      crate::fsutil::read_json(&f).with_context(|| format!("解析 {} 失败", f.display()))?;
    for (k, v) in map {
      let value = serde_json::from_value(v)
        .with_context(|| format!("{} 的 {k} 不是字符串或复数对象", f.display()))?;
      out.push(Entry {
        key: k,
        value,
        domain: domain.clone(),
      });
    }
  }
  Ok(out)
}

/// 中文词典（key → 中文原文），供导出待译模板时带上原文作参考。
///
/// 中文不使用复数变体，因此取兜底文本即可。
pub fn zh_map(root: &Path) -> Result<HashMap<String, String>> {
  Ok(
    load_entries(root, "zh")?
      .into_iter()
      .map(|e| (e.key, e.value.text().to_owned()))
      .collect(),
  )
}

/// 中文原文 → 语义 key 的反查表。
///
/// 正常写法是 `tf("settings.merged-items", …)`，但也有调用点直接把中文原文当 key
/// 写（`tf("已合并 {} 条数据", …)`）—— 这类调用点在只认语义 key 的扫描里是纯盲区：
/// 它既进不了 [`crate::i18n::plural_candidates`] 的候选表，也进不了 `tp` 对账，
/// 于是「英文里写 `1 datas`」这种错就一直挂着没人发现。
///
/// 判定复用 [`unused`] 已有的「源码字面量包含中文原文」思路，只是方向反过来：
/// 由调用点的字面量反查它属于哪条词条。
struct ZhIndex {
  /// 字面量 == 中文原文：O(1) 精确命中。
  exact: HashMap<String, String>,
  /// 字面量包含中文原文（`format!` 把原文拼进更长句子的情况）。
  /// 按原文长度降序，命中时取最长者，避免「{} 条」抢在「已合并 {} 条数据」前面。
  fuzzy: Vec<(String, String)>,
}

impl ZhIndex {
  fn new(root: &Path) -> Result<Self> {
    Ok(Self::from_zh(zh_map(root)?))
  }

  fn from_zh(zh: HashMap<String, String>) -> Self {
    let mut exact = HashMap::with_capacity(zh.len());
    let mut fuzzy = Vec::new();
    for (key, text) in zh {
      // 只有带 `{}` 的原文才值得做包含匹配：没有占位符的文案进不了复数候选
      // （见 [`candidates_of`]），拿它当子串只会让「条」「次」这类短原文命中一切。
      if count_placeholders(&text) > 0 {
        fuzzy.push((text.clone(), key.clone()));
      }
      exact.insert(text, key);
    }
    // 按原文长度降序；等长时再按原文排序，否则 `HashMap` 的随机迭代顺序会让
    // 等长并列的命中结果在两次运行间抖动（校验输出不可复现）。
    fuzzy.sort_by(|a, b| {
      b.0
        .chars()
        .count()
        .cmp(&a.0.chars().count())
        .then_with(|| a.0.cmp(&b.0))
    });
    Self { exact, fuzzy }
  }

  /// 调用点的字面量 → 语义 key；解析不出来就原样返回（本来就是语义 key）。
  fn resolve(&self, lit: &str) -> String {
    if let Some(key) = self.exact.get(lit) {
      return key.clone();
    }
    // 语义 key 不含 CJK：先按这个判据短路，省掉 O(n) 的包含匹配。
    if has_cjk(lit)
      && let Some((_, key)) = self.fuzzy.iter().find(|(zh, _)| lit.contains(zh.as_str()))
    {
      return key.clone();
    }
    lit.to_owned()
  }
}

/// 校验结果。
#[derive(Debug, Default)]
pub struct Report {
  /// 同一语言内重复出现的 key（跨域重复同样算，后写的会覆盖先写的）。
  pub duplicates: Vec<String>,
  /// 译文里 `{}` 数量与中文不一致（复数变体记作 `key#类别`）。
  pub placeholder_mismatch: Vec<(String, usize, usize)>,
  /// key 缺少 `<domain>.` 前缀，或前缀与所在域文件不一致。
  pub bad_namespace: Vec<String>,
  /// 复数词条的问题：变体名不合法、缺 `other`、或中文用了复数变体。
  pub bad_plural: Vec<String>,
  /// 一句话里塞了 ≥ 2 处「数量 + 可数名词」：`tp()` 只吃一个 `count`，管不了。
  pub multi_count: Vec<String>,
}

/// 校验一种语言的词典（`zh_map` 是占位符比对的基准；校验 zh 自身时传同一份即可）。
///
/// 复数词条的每个变体都与中文原文单独比对占位符 —— 中文是扁平的，`{}` 个数以它为准。
pub fn check(entries: &[Entry], zh_map: &HashMap<String, String>, lang: &str) -> Report {
  let mut seen: HashSet<&str> = HashSet::new();
  let mut r = Report::default();
  for e in entries {
    if !seen.insert(e.key.as_str()) {
      r.duplicates.push(e.key.clone());
    }
    if e.key.split('.').next() != Some(e.domain.as_str()) {
      r.bad_namespace.push(e.key.clone());
    }
    let base = zh_map.get(&e.key).map_or(0, |v| count_placeholders(v));
    match &e.value {
      Value::Plain(v) => {
        let tr = count_placeholders(v);
        if base != tr {
          r.placeholder_mismatch.push((e.key.clone(), base, tr));
        }
      }
      Value::Plural(variants) => {
        if lang == "zh" {
          r.bad_plural
            .push(format!("{}：中文不使用复数变体，请写成字符串", e.key));
        }
        for (category, text) in variants {
          if ham_web_core::plural::Category::from_variant(category).is_none() {
            r.bad_plural
              .push(format!("{}#{}：不是合法的复数类别", e.key, category));
          }
          let tr = count_placeholders(text);
          if base != tr {
            r.placeholder_mismatch
              .push((format!("{}#{}", e.key, category), base, tr));
          }
        }
        if !variants.contains_key("other") {
          r.bad_plural
            .push(format!("{}：复数变体缺少 other（兜底类别）", e.key));
        }
      }
    }
  }
  // 中文不变形，多计数对 zh 无害；只守 en / es。
  if lang != "zh" {
    let (nouns, singulars) = countable_nouns(entries, lang);
    for e in entries {
      if let Value::Plain(v) = &e.value {
        let n = count_noun_slots(v, &nouns, &singulars);
        if n >= 2 {
          r.multi_count.push(format!(
            "{}：{} 处「数量 + 可数名词」（`tp()` 只吃一个 count，需按 P3-C5 改写） → {v}",
            e.key, n
          ));
        }
      }
    }
  }
  r
}

/// 一句话里「`{}` 后紧跟可数名词」的处数。
///
/// 只数紧邻的情况（与 e2e 盯 `1 <复数名词>` 的口径一致）：`{} new questions`
/// 这种中间隔了修饰语的漏掉可以接受 —— 这条检查是**守门**，宁可漏报也不误报；
/// 真漏了的会在 e2e 的 count = 1 扫描里现形。
fn count_noun_slots(text: &str, nouns: &HashSet<String>, singulars: &HashSet<String>) -> usize {
  let cs: Vec<char> = text.chars().collect();
  let mut n = 0;
  let mut i = 0;
  while i + 1 < cs.len() {
    if cs[i] == '{' && cs[i + 1] == '}' {
      let mut j = i + 2;
      while j < cs.len() && !cs[j].is_alphabetic() {
        j += 1;
      }
      let word: String = cs[j..].iter().take_while(|c| c.is_alphabetic()).collect();
      if !word.is_empty() && is_countable(&word, nouns, singulars) {
        n += 1;
      }
      i = j;
    } else {
      i += 1;
    }
  }
  n
}

/// 可数名词词表：`(所有词形, 会变形的单数形式)`。
///
/// 词表**自维护**：从本语言已迁的复数变体里取每个变体的最后一个词，迁一条多一词
/// （单复数两形都在）。`QSOs` 这类还没迁复数词条的派生不出来，用 [`EXTRA_COUNTABLE`] 兜底。
///
/// 「会变形」是过滤噪音的关键：只有单复数两形都出现过的词才算可数名词，
/// `correct` / `answered` / `minute` 这类没有复数形式的词因此被滤掉。
fn countable_nouns(entries: &[Entry], lang: &str) -> (HashSet<String>, HashSet<String>) {
  let mut nouns: HashSet<String> = HashSet::new();
  for e in entries {
    if let Value::Plural(variants) = &e.value {
      for text in variants.values() {
        if let Some(word) = last_word(text) {
          nouns.insert(word.to_lowercase());
        }
      }
    }
  }
  for (l, words) in EXTRA_COUNTABLE {
    if *l != lang {
      continue;
    }
    for w in *words {
      let w = w.to_lowercase();
      nouns.insert(w.clone());
      nouns.insert(format!("{w}s"));
      nouns.insert(format!("{w}es"));
    }
  }
  let singulars: HashSet<String> = nouns
    .iter()
    .filter(|w| nouns.contains(&format!("{w}s")) || nouns.contains(&format!("{w}es")))
    .cloned()
    .collect();
  (nouns, singulars)
}

/// 兜底名词（单数形式）：e2e `i18n_plural.spec.ts` 盯的那批核心可数名词。
///
/// 有些还没迁复数词条（如 `QSO`），派生不出来，先写死 —— 它们正是历史上出过
/// 多计数文案的那批（P3-C5 实测的 4 条全在这张表里）。
const EXTRA_COUNTABLE: &[(&str, &[&str])] = &[
  (
    "en",
    &[
      "day",
      "question",
      "card",
      "item",
      "match",
      "mistake",
      "record",
      "QSO",
      "grid",
      "label",
      "page",
      "hour",
      "time",
      "attempt",
      "point",
      "line",
      "entry",
      "zone",
      "square",
      "entity",
      "pass",
      "month",
      "stage",
      "report",
      "countdown",
      "callsign",
    ],
  ),
  (
    "es",
    &[
      "día",
      "pregunta",
      "tarjeta",
      "elemento",
      "coincidencia",
      "error",
      "registro",
      "cuadrícula",
      "etiqueta",
      "página",
      "hora",
      "vez",
      "intento",
      "punto",
      "línea",
      "entrada",
      "zona",
      "contactado",
      "entidad",
      "paso",
      "mes",
      "etapa",
      "informe",
      "cuenta",
      "indicativo",
    ],
  ),
];

/// 判断词形是否可数：词形在词表里，且它（或去掉 `s` / `es` 的词干）会变形。
fn is_countable(word: &str, nouns: &HashSet<String>, singulars: &HashSet<String>) -> bool {
  let w = word.to_lowercase();
  if !nouns.contains(&w) {
    return false;
  }
  if singulars.contains(&w) {
    return true;
  }
  ["es", "s"].iter().any(|suffix| {
    w.strip_suffix(suffix)
      .is_some_and(|stem| singulars.contains(stem))
  })
}

/// 取文案的最后一个词（复数变体里 `{} days` 的 `days` 就是要收的词）。
fn last_word(text: &str) -> Option<String> {
  let cleaned = text.replace("{}", " ");
  let word = cleaned
    .split_whitespace()
    .next_back()?
    .trim_matches(|c: char| !c.is_alphabetic());
  (!word.is_empty() && word.chars().all(char::is_alphabetic)).then(|| word.to_owned())
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

/// 读入源码，并涂白注释与测试模块（见 [`mask_comments`] / [`mask_test_modules`]）。
///
/// 顺序是先注释后测试模块：注释涂白后不剩花括号，`mask_test_modules` 的配对因此更准。
fn read_source(f: &Path) -> Result<String> {
  Ok(mask_test_modules(&mask_comments(&std::fs::read_to_string(
    f,
  )?)))
}

/// 枚举源码里出现的所有 `t("…")` / `tf("…")` / `tp("…")` 的 key。
///
/// 写中文原文的调用（`tf("已合并 {} 条数据", …)`）会被 [`ZhIndex`] 解析回语义 key，
/// 否则它们会以中文的身份出现在「词典里没有的 key」里，制造假的缺口。
pub fn collect_call_sites(root: &Path) -> Result<HashSet<String>> {
  let zh = ZhIndex::new(root)?;
  let mut out = HashSet::new();
  for entry in walk(&root.join("crates/app/src"))? {
    let src = read_source(&entry)?;
    for (_, _, key) in scan_calls(&src) {
      out.insert(zh.resolve(&key));
    }
  }
  Ok(out)
}

/// 枚举源码里按 `tp("…", n, …)` 调用的 key —— 这些文案声明了「要按数量换词形」。
///
/// 同样经过 [`ZhIndex`] 解析：`tp("已恢复 {} 条数据", n, …)` 这类写法也算数，
/// 不然「改成了 `tp` 却没配变体」的对账会把它们漏掉。
pub fn collect_plural_call_sites(root: &Path) -> Result<HashSet<String>> {
  let zh = ZhIndex::new(root)?;
  let mut out = HashSet::new();
  for entry in walk(&root.join("crates/app/src"))? {
    let src = read_source(&entry)?;
    for (_, kind, key) in scan_calls(&src) {
      if kind == "tp" {
        out.insert(zh.resolve(&key));
      }
    }
  }
  Ok(out)
}

/// `key → 调用点（相对路径:行号）`，只收 `tf(` / `tp(` —— 复数候选必须有占位符参数，
/// 报告里也靠行号人工确认 `tp` 的 count 对应哪个占位符。
///
/// 中文原文作 key 的调用点会解析成语义 key 后合并进来（见 [`ZhIndex`]）：
/// 不解析的话它们永远不在候选表上，成了纯静态扫描的盲区。
fn call_locations(root: &Path) -> Result<HashMap<String, Vec<String>>> {
  let zh = ZhIndex::new(root)?;
  let mut out: HashMap<String, Vec<String>> = HashMap::new();
  for f in walk(&root.join("crates/app/src"))? {
    let src = read_source(&f)?;
    let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
    for (offset, kind, key) in scan_calls(&src) {
      // `t` / `set_title` 没有占位符实参，谈不上复数。
      if !matches!(kind, "tf" | "tp") {
        continue;
      }
      let key = zh.resolve(&key);
      let line = src[..offset].matches('\n').count() + 1;
      out.entry(key).or_default().push(format!("{rel}:{line}"));
    }
  }
  for calls in out.values_mut() {
    calls.sort();
    calls.dedup();
  }
  Ok(out)
}

/// 调用的实参区间：从 `(`（下标 `open`）到配对的 `)` 之后。
///
/// 只看括号配对与字符串字面量 —— 注释在 [`read_source`] 里已经涂白了。
fn call_args_end(src: &str, open: usize) -> Option<usize> {
  let bytes = src.as_bytes();
  if bytes.get(open) != Some(&b'(') {
    return None;
  }
  let mut depth = 0usize;
  let mut i = open;
  while i < bytes.len() {
    match bytes[i] {
      b'(' => {
        depth += 1;
        i += 1;
      }
      b')' => {
        depth -= 1;
        i += 1;
        if depth == 0 {
          return Some(i);
        }
      }
      b'"' | b'\'' | b'r' | b'b' => i = literal_end(bytes, i).map_or(i + 1, |e| e.max(i + 1)),
      _ => i += 1,
    }
  }
  None
}

/// `[from, to)` 区间里的全部字符串字面量（内容 + 起始字节下标）。
fn literals_at(src: &str, from: usize, to: usize) -> Vec<(String, usize)> {
  let bytes = src.as_bytes();
  let mut out = Vec::new();
  let mut i = from;
  while i < to && i < bytes.len() {
    if bytes[i] == b'"' {
      match read_string(src, i) {
        Some((s, next)) => {
          out.push((s, i));
          i = next.max(i + 1);
        }
        None => i += 1,
      }
    } else {
      i += 1;
    }
  }
  out
}

/// 是否 CJK 表意文字 / 假名（**不含** CJK 标点与全角符号）。
///
/// 只有这类字符出现在 `tf` / `tp` 的实参里才算漏翻译：`join("、")` 这类分隔符是
/// 导出文本的约定（`、` 不是字母），不该被当成漏掉的文案。
fn is_cjk_letter(c: char) -> bool {
  let u = c as u32;
  (0x3040..=0x30FF).contains(&u)        // 平假名 / 片假名
    || (0x3400..=0x4DBF).contains(&u)   // CJK 扩展 A
    || (0x4E00..=0x9FFF).contains(&u)   // CJK 基本区
    || (0xF900..=0xFAFF).contains(&u)   // 兼容表意文字
    || (0x2_0000..=0x2_FA1F).contains(&u) // 扩展 B 及以后
}

/// 第一个实参的结束位置（该实参之后那个「顶层逗号」的下标；没有逗号就是 `end`）。
///
/// 只数括号深度，并跳过字符串字面量。用来判断某个字面量是否落在**第一个实参**里 —
/// 那是 key 的位置（`t` / `tf` / `tp` 都会经 `catalog::resolve` 走中文反向索引），
/// 而后面的实参只是被 `substitute` 按 `{}` 顺序填进去，写中文就会原样漏到 en / es 界面。
fn first_arg_end(src: &str, open: usize, end: usize) -> usize {
  let bytes = src.as_bytes();
  let mut depth = 0i32;
  let mut i = open + 1;
  while i < end {
    match bytes[i] {
      b'(' | b'[' | b'{' => depth += 1,
      b')' | b']' | b'}' => depth -= 1,
      b'"' => {
        // 跳过整个字符串字面量（含 `\` 转义）。
        i += 1;
        while i < end && bytes[i] != b'"' {
          if bytes[i] == b'\\' {
            i += 1;
          }
          i += 1;
        }
      }
      b',' if depth == 0 => return i,
      _ => {}
    }
    i += 1;
  }
  end
}

/// `tf` / `tp` 实参里写中文字面量的调用点（`相对路径:行号：字面量`）。
///
/// `t()` 的入参可以是中文原文（走 `catalog::resolve` 的反向索引，属受支持的兜底写法），
/// 但 `tf` / `tp` 的实参只是被 `substitute` 按 `{}` 顺序填进去，**没有反向索引** ——
/// 实参里写中文，en / es 界面就会漏出中文：线上那句 `order 4 (n 阶)` 就是这么来的。
///
/// 判定「是不是 key」用**实参位置**，不能数字面量序号：`tf(label, "中文")` 的首参不是
/// 字面量，`literals_at` 返回的第一个字面量其实是那个中文实参，用 `n == 0` 跳过它，
/// 正好把这条规则该报的情况漏掉（`unresolved_calls` 只提示「首参非字面量」，是另一份清单）。
fn cjk_args(root: &Path) -> Result<Vec<String>> {
  let mut out = Vec::new();
  for f in walk(&root.join("crates/app/src"))? {
    let src = read_source(&f)?;
    let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
    let calls = scan_calls(&src);
    // 每一处 `t(` / `tf(` / `tp(` 的实参区间：用来识别「裸写在实参里」与
    // 「套在内层 `t()` 里」两种写法。不能用 `scan_calls` 的结果来算 ——
    // `t(if passed { "合格" } …)` 的首个实参不是字面量，它压根不在 `scan_calls` 里。
    let regions = call_regions(&src);
    for (offset, kind, _) in &calls {
      if *kind == "t" {
        continue;
      }
      // `scan_calls` 给的偏移是函数名起点，`(` 在 `offset + kind.len()` 处。
      let open = offset + kind.len();
      let Some(end) = call_args_end(&src, open) else {
        continue;
      };
      let key_end = first_arg_end(&src, open, end);
      let line = src[..*offset].matches('\n').count() + 1;
      for (lit, at) in literals_at(&src, open, end) {
        if at < key_end || !lit.chars().any(is_cjk_letter) {
          continue;
        }
        // 套在内层 `t(…)` 里的中文是**受支持**的兜底写法（`t()` 会走 `catalog::resolve`
        // 的反向索引，`share_score.rs` 的 `&t(if passed { "合格" } else { "不合格" })`
        // 就是这类）；只有裸写在实参里的才会原样漏到 en / es 界面上。
        let wrapped = regions
          .iter()
          .any(|&(o2, e2)| o2 > open && o2 <= at && at < e2);
        if !wrapped {
          out.push(format!("{rel}:{line}：{lit}"));
        }
      }
    }
  }
  out.sort();
  out.dedup();
  Ok(out)
}

/// 首个实参不是字符串字面量的调用点（`t(&format!(…))` / `t(SOME_CONST)` / `tf(label, …)`）。
///
/// 这类调用点纯静态扫描认不出来，对应的 key 既可能被误判成死条目，也可能不在覆盖率里。
/// 工具无法自动解析，只能列出来提示人工核对。
fn unresolved_calls(root: &Path) -> Result<Vec<String>> {
  let mut out = Vec::new();
  for f in walk(&root.join("crates/app/src"))? {
    let src = read_source(&f)?;
    let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if bytes[i] == b't'
        && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
        && let Some(open) = call_open(bytes, i)
      {
        let mut j = open + 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
          j += 1;
        }
        if bytes.get(j) != Some(&b'"') {
          let line = src[..i].matches('\n').count() + 1;
          out.push(format!("{rel}:{line}"));
        }
      }
      i += 1;
    }
  }
  out.sort();
  out.dedup();
  Ok(out)
}

/// 源码里每一处 `t(` / `tf(` / `tp(` 的实参区间 `(左括号, 右括号之后)`。
///
/// 与 [`scan_calls`] 的区别是**不要求首个实参是字面量**：`t(if passed { "合格" } …)`
/// 也算一处调用，[`cjk_args`] 靠它区分「裸写在实参里」与「套在内层 `t()` 里」。
fn call_regions(src: &str) -> Vec<(usize, usize)> {
  let bytes = src.as_bytes();
  let mut out = Vec::new();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b't'
      && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
      && let Some(open) = call_open(bytes, i)
      && let Some(end) = call_args_end(src, open)
    {
      out.push((open, end));
    }
    i += 1;
  }
  out
}

/// `i` 处是 `t(` / `tf(` / `tp(` / `set_title(` 时返回左括号的下标。
fn call_open(bytes: &[u8], i: usize) -> Option<usize> {
  if bytes[i..].starts_with(b"set_title(") {
    return Some(i + 9);
  }
  match (bytes.get(i + 1), bytes.get(i + 2)) {
    (Some(b'('), _) => Some(i + 1),
    (Some(b'f') | Some(b'p'), Some(b'(')) => Some(i + 2),
    _ => None,
  }
}

/// key 是否是「slug 冲突自动加序号」的产物：末段以 `-数字` 结尾，且去掉该后缀后
/// **确实存在**同名的 key（`common.correct-2` ← `common.correct`）。
///
/// 后半个条件很重要：`tools.recommended-1024-768-4096` 这类数字本来就有语义，
/// 不能算冲突产物。
fn is_collision_slug(key: &str, all: &HashSet<String>) -> bool {
  let Some((head, slug)) = key.rsplit_once('.') else {
    return false;
  };
  let Some((base, tail)) = slug.rsplit_once('-') else {
    return false;
  };
  !tail.is_empty()
    && tail.bytes().all(|b| b.is_ascii_digit())
    && all.contains(&format!("{head}.{base}"))
}

/// 存量「编号 slug」清单（150 条）：slug 由英文译文派生，冲突时自动加 `-数字`，
/// 语义为零（`common.correct-2` / `-3` / `-4` 谁都看不出区别）。
///
/// **这就是重命名清单**：改掉一条就从这里删一条，`check-i18n` 会提示清单里哪些
/// 已经不存在了。新增编号 slug 一律判失败 —— 别让第 151 条再长出来。
const NUMBERED_SLUG_BASELINE: &[&str] = &[
  "common.accuracy-2",
  "common.ci-v-address-hex-2",
  "common.class-2",
  "common.correct-2",
  "common.correct-3",
  "common.correct-4",
  "common.days-until-the-class-2",
  "common.entry-2",
  "common.preview-2",
  "common.questions-2",
  "common.questions-3",
  "common.review-2",
  "common.speed-wpm-2",
  "common.take-one-2",
  "common.wrong-2",
  "common.zone-2",
  "contest.10-m-band-only-2",
  "contest.24-hour-field-emergency-2",
  "contest.contest-2",
  "contest.contest-tips-2",
  "contest.exchange-2",
  "contest.global-rtty-contest-exchange-2",
  "contest.in-about-months-2",
  "contest.received-2",
  "contest.the-ssb-leg-of-2",
  "contest.the-ssb-leg-of-3",
  "contest.the-ssb-leg-of-4",
  "contest.the-world-s-biggest-2",
  "contest.this-month-2",
  "exam.answered-2",
  "exam.class-2",
  "exam.class-3",
  "exam.correct-2",
  "exam.correct-3",
  "exam.correct-4",
  "exam.correct-answer-2",
  "exam.high-frequency-exam-points-2",
  "exam.loading-questions-2",
  "exam.multiple-2",
  "exam.no-2",
  "exam.ok-2",
  "exam.practise-similar-2",
  "exam.previous-2",
  "exam.question-2",
  "exam.questions-2",
  "exam.remaining-2",
  "exam.remember-2",
  "exam.resume-2",
  "exam.score-2",
  "exam.submit-2",
  "exam.unanswered-2",
  "exam.wrong-2",
  "knowledge.iaru-region-3-band-2",
  "knowledge.meaning-2",
  "knowledge.velocity-factor-k-0-2",
  "knowledge.which-band-does-this-2",
  "learning.accuracy-2",
  "learning.correct-2",
  "learning.data-comes-from-local-2",
  "learning.don-t-know-2",
  "learning.fri-2",
  "learning.mon-2",
  "learning.progress-2",
  "learning.sat-2",
  "learning.sun-2",
  "learning.thu-2",
  "learning.tue-2",
  "learning.wed-2",
  "log.entry-2",
  "log.grid-2",
  "log.iota-island-groups-2",
  "log.page-2",
  "log.records-2",
  "log.records-3",
  "log.station-grid-2",
  "log.was-us-states-2",
  "log.worked-2",
  "morse.character-2",
  "morse.frequent-misses-2",
  "morse.play-2",
  "morse.replay-2",
  "morse.streak-2",
  "morse.volume-2",
  "radio.altitude-2",
  "radio.band-name-2",
  "radio.callsign-or-entity-name-2",
  "radio.centre-lat-lon-2",
  "radio.coloured-by-each-dxcc-2",
  "radio.country-region-2",
  "radio.cq-itu-zone-map-2",
  "radio.data-from-the-public-2",
  "radio.e-g-bg4xyz-ja1abc-2",
  "radio.enter-a-callsign-to-2",
  "radio.enter-a-callsign-to-3",
  "radio.entry-2",
  "radio.key-concepts-2",
  "radio.not-worked-2",
  "radio.notes-2",
  "radio.reference-2",
  "radio.scroll-pinch-to-zoom-2",
  "radio.status-2",
  "radio.watched-2",
  "radio.worked-2",
  "radio.x-ray-flux-2",
  "radio.zone-2",
  "radio.zone-3",
  "shell.amateur-radio-exams-knowledge-2",
  "shell.amateur-tv-2",
  "shell.antenna-analyzers-2",
  "shell.antenna-arrays-2",
  "shell.antenna-farms-2",
  "shell.antenna-modeling-2",
  "shell.ardf-2",
  "shell.band-chart-2",
  "shell.beginner-s-guide-2",
  "shell.cabrillo-logs-2",
  "shell.dx-awards-2",
  "shell.emergency-comms-2",
  "shell.exam-reference-2",
  "shell.iota-2",
  "shell.packet-radio-2",
  "shell.q-codes-2",
  "shell.qrp-2",
  "shell.receiver-specs-2",
  "shell.repeater-building-2",
  "shell.repeaters-gateways-2",
  "shell.rfi-troubleshooting-2",
  "shell.search-2",
  "shell.software-defined-radio-2",
  "shell.special-propagation-2",
  "tools.air-core-coil-inductance-2",
  "tools.click-to-choose-or-2",
  "tools.could-not-read-the-2",
  "tools.decoding-failed-make-sure-2",
  "tools.enter-a-positive-frequency-2",
  "tools.enter-a-positive-power-2",
  "tools.file-too-large-about-2",
  "tools.file-too-large-about-3",
  "tools.free-space-path-loss-2",
  "tools.inductance-h-2",
  "tools.pulse-shaping-roll-off-2",
  "tools.receiver-sensitivity-2",
  "tools.resistance-r-2",
  "tools.series-reactance-2",
  "tools.signal-to-noise-ratio-2",
  "tools.signal-to-noise-ratio-3",
  "tools.swr-2",
  "tools.symmetrical-t-resistive-attenuators-2",
  "tools.the-amber-dashed-line-2",
  "tools.utc-time-2",
];

fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
  let mut out = Vec::new();
  let mut stack = vec![dir.to_path_buf()];
  while let Some(d) = stack.pop() {
    for e in std::fs::read_dir(&d).with_context(|| format!("读取目录 {} 失败", d.display()))?
    {
      let p = e?.path();
      if p.is_dir() {
        stack.push(p);
      } else if p.extension().is_some_and(|x| x == "rs") {
        out.push(p);
      }
    }
  }
  out.sort();
  Ok(out)
}

/// 提取源码中的文案调用：`(字节偏移, 函数名, key)`（只处理不含转义的简单字面量）。
///
/// 函数名随结果返回：`tp("…")` 是「这条文案要按数量换词形」的证据，`check-i18n`
/// 据此对账词典里有没有配复数变体；偏移量用来算行号，候选报告里好定位。
fn scan_calls(src: &str) -> Vec<(usize, &'static str, String)> {
  let mut out = Vec::new();
  let bytes = src.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    // 找 t( / tf( / tp( / set_title(；要求名字不是更长标识符的结尾，否则 `split("…")` /
    // `attempt("…")` 这类以 `t` 结尾的调用会被误当成文案调用。
    //
    // `set_title("shell.x")` 与 `set_title_with_args("radio.print", …)` 也要认：它们存的是
    // **语义 key**（由 `apply_title` 在切语言时再查表），字面量前面没有 `t(` —— 不认它，
    // 那个 key 就会被判成死条目。
    let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
    let kind: Option<&'static str> = if !boundary {
      None
    } else if bytes[i..].starts_with(b"set_title_with_args(") {
      // 必须排在 `set_title(` 前面：后者要求紧跟 `(`，长名字不会被它误命中。
      Some("set_title_with_args")
    } else if bytes[i..].starts_with(b"set_title(") {
      Some("set_title")
    } else if bytes[i..].starts_with(b"tf(") {
      Some("tf")
    } else if bytes[i..].starts_with(b"tp(") {
      Some("tp")
    } else if bytes[i..].starts_with(b"t(") {
      Some("t")
    } else {
      None
    };
    if let Some(name) = kind {
      // 跳过 `( ` 之后的空白（含换行）：rustfmt 会把长调用拆成多行，
      // ```text
      // tf(
      //   "a.b",
      //   &[…],
      // )
      // ```
      // 不跳过换行就会整条漏掉 —— 这些 key 明明在用，却会被判成死条目。
      let mut open = i + name.len() + 1;
      while open < bytes.len() && bytes[open].is_ascii_whitespace() {
        open += 1;
      }
      // 注意：解析失败时不能 `continue`，否则跳过末尾的 `i += 1` 导致死循环。
      let lit = read_string(src, open)
        .map(|(s, _)| s)
        // `set_title` 既接受语义 key 也接受裸标题（`APRS` / `RTTY / PSK31` / 中文原名），
        // 只有 key 形态的才算「这个 key 还在用」，否则覆盖率会报出一堆不存在的「待补」。
        .filter(|s| name != "set_title" || looks_like_key(s));
      if let Some(s) = lit {
        out.push((i, name, s));
      }
    }
    i += 1;
  }
  out
}

/// 字面量是否是语义 key 的形态（`<域>.<词条>`）：`set_title` 的入参两种都可能是。
fn looks_like_key(lit: &str) -> bool {
  let Some((domain, slug)) = lit.split_once('.') else {
    return false;
  };
  !slug.is_empty()
    && !has_cjk(lit)
    && !domain.is_empty()
    && domain
      .bytes()
      .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// 把 `#[cfg(test)] mod … { … }` 整块涂成空格（换行保留）。
///
/// 涂白而不是删除：字节偏移与行号都不变，报告里的 `文件:行号` 依旧准确。
///
/// 测试里构造的调用不是真实用法 —— `tp("shell.home", 1, &[])` 这种只存在于单测的
/// 调用会让「已用 `tp()` 但词典没配变体」的对账出现假阳性。
fn mask_test_modules(src: &str) -> String {
  const MARK: &[u8] = b"#[cfg(test)]";
  let bytes = src.as_bytes();
  let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i..].starts_with(MARK) {
      let mut j = i + MARK.len();
      while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
      }
      // `mod tests { … }` 或 `mod tests\n{ … }`
      if bytes[j..].starts_with(b"mod") {
        let mut k = j + 3;
        while k < bytes.len() && bytes[k] != b'{' && bytes[k] != b';' {
          k += 1;
        }
        if k < bytes.len() && bytes[k] == b'{' {
          let end = block_end(src, k);
          for &b in &bytes[i..end] {
            out.push(if b == b'\n' { b'\n' } else { b' ' });
          }
          i = end;
          continue;
        }
      }
    }
    out.push(bytes[i]);
    i += 1;
  }
  String::from_utf8(out).unwrap_or_else(|_| src.to_owned())
}

/// 把注释涂成空格（换行保留），逻辑与 [`mask_test_modules`] 同源：涂白而非删除，
/// 字节偏移与行号都不变。
///
/// 为什么必须做：[`scan_calls`] 是纯文本扫描，文档注释里举例写的
/// `t("exam.search-questions")`（见 `crates/app/src/ui/control.rs`）会被当成真实调用点 ——
/// 那个 key 于是永远查不出死条目；反过来注释里写个不存在的 key 会以「待补」污染覆盖率。
/// [`unused`] 走的 `source_literals()` 本来就剥注释，两条路径必须同口径。
fn mask_comments(src: &str) -> String {
  let bytes = src.as_bytes();
  let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      // 行注释：`//` / `///` / `//!`，涂到行尾（换行保留）
      b'/' if bytes.get(i + 1) == Some(&b'/') => {
        while i < bytes.len() && bytes[i] != b'\n' {
          out.push(b' ');
          i += 1;
        }
      }
      // 块注释：Rust 允许嵌套
      b'/' if bytes.get(i + 1) == Some(&b'*') => {
        let mut depth = 0usize;
        while i < bytes.len() {
          let (step, closed) = if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            depth += 1;
            (2, false)
          } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
            depth -= 1;
            (2, depth == 0)
          } else {
            (1, false)
          };
          // 涂白的字节数必须与吃掉的字节数严格相等（末尾截断时也不能多涂），
          // 否则后面的偏移与行号会整体漂移。
          let n = step.min(bytes.len() - i);
          for k in 0..n {
            out.push(if bytes[i + k] == b'\n' { b'\n' } else { b' ' });
          }
          i += n;
          if closed {
            break;
          }
        }
      }
      // 字符串 / 字符 / 原始字符串 / 字节串：整段照抄，否则 `"http://…"` 里的 `//`
      // 会被当注释起点，把后面的真实代码涂掉。
      b'"' | b'\'' | b'r' | b'b' => match literal_end(bytes, i) {
        Some(end) => {
          out.extend_from_slice(&bytes[i..end]);
          i = end;
        }
        None => {
          out.push(bytes[i]);
          i += 1;
        }
      },
      b => {
        out.push(b);
        i += 1;
      }
    }
  }
  String::from_utf8(out).unwrap_or_else(|_| src.to_owned())
}

/// `i` 处若是字面量（`"…"` / `'…'` / `r"…"` / `r#"…"#` / `b"…"` / `br#"…"#`），
/// 返回结束位置（闭合引号之后的字节下标）；否则 `None`（如 `bit` / `'a` 生命周期）。
fn literal_end(bytes: &[u8], i: usize) -> Option<usize> {
  // 字符字面量：`'x'` / `'\n'` / `'\u{4e00}'`。只认能闭合的，`'a`（生命周期）不认。
  if bytes[i] == b'\'' {
    if bytes.get(i + 1) == Some(&b'\\') {
      let mut j = i + 2;
      while j < bytes.len() && j <= i + 12 {
        if bytes[j] == b'\'' {
          return Some(j + 1);
        }
        j += 1;
      }
      return None;
    }
    return (bytes.get(i + 2) == Some(&b'\'')).then_some(i + 3);
  }
  let mut j = i;
  if bytes.get(j) == Some(&b'b') {
    j += 1;
  }
  let raw = bytes.get(j) == Some(&b'r');
  if raw {
    j += 1;
    let mut hashes = 0usize;
    while bytes.get(j) == Some(&b'#') {
      hashes += 1;
      j += 1;
    }
    if bytes.get(j) != Some(&b'"') {
      return None;
    }
    j += 1;
    while j < bytes.len() {
      if bytes[j] == b'"' {
        let mut k = j + 1;
        let mut n = 0usize;
        while n < hashes && bytes.get(k) == Some(&b'#') {
          n += 1;
          k += 1;
        }
        if n == hashes {
          return Some(k);
        }
      }
      j += 1;
    }
    return Some(bytes.len());
  }
  if bytes.get(j) != Some(&b'"') {
    return None;
  }
  j += 1;
  while j < bytes.len() {
    match bytes[j] {
      b'\\' => j += 2,
      b'"' => return Some(j + 1),
      _ => j += 1,
    }
  }
  Some(bytes.len())
}

/// 从 `{`（下标 `open`）起找到配对的 `}` 之后的位置；跳过字符串与行注释里的括号。
fn block_end(src: &str, open: usize) -> usize {
  let bytes = src.as_bytes();
  let mut depth = 0usize;
  let mut i = open;
  while i < bytes.len() {
    match bytes[i] {
      b'{' => {
        depth += 1;
        i += 1;
      }
      b'}' => {
        depth -= 1;
        i += 1;
        if depth == 0 {
          return i;
        }
      }
      b'"' => match read_string(src, i) {
        Some((_, next)) => i = next,
        None => i += 1,
      },
      b'/' if bytes.get(i + 1) == Some(&b'/') => {
        while i < bytes.len() && bytes[i] != b'\n' {
          i += 1;
        }
      }
      _ => i += 1,
    }
  }
  bytes.len()
}

/// 从 `i`（必须是 `"`）开始读取字符串字面量，返回内容与结束位置。
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

/// 处理常见转义（`\"` `\\` `\n` `\'` `\0` `\xNN` `\u{…}`）。
///
/// 未知转义返回 `None`（调用方会跳过这条字面量）：这类源码本身编译不过，跳过比
/// 猜一个字符更安全。`\xNN` / `\u{…}` 必须认出来 —— 漏掉它们会让带这类转义的调用点
/// 整条被跳过，对应的 key 于是被判成「死条目」、覆盖率也漏算（口径见
/// `crates/tools/src/knowledge_i18n.rs` 的同名函数）。
fn unescape(s: &str, i: usize) -> Option<(char, usize)> {
  if i + 1 >= s.len() {
    return None;
  }
  match s.as_bytes()[i + 1] {
    b'"' => Some(('"', i + 2)),
    b'\\' => Some(('\\', i + 2)),
    b'n' => Some(('\n', i + 2)),
    b'r' => Some(('\r', i + 2)),
    b't' => Some(('\t', i + 2)),
    b'\'' => Some(('\'', i + 2)),
    b'0' => Some(('\0', i + 2)),
    // `\xNN`：恰好两位十六进制。
    b'x' => {
      let v = u8::from_str_radix(s.get(i + 2..i + 4)?, 16).ok()?;
      Some((char::from(v), i + 4))
    }
    // `\u{…}`：1–6 位十六进制。
    b'u' => {
      let rest = s.get(i + 2..)?.strip_prefix('{')?;
      let end = rest.find('}')?;
      let v = u32::from_str_radix(&rest[..end], 16).ok()?;
      Some((char::from_u32(v)?, i + 4 + end))
    }
    _ => None,
  }
}

/// 取源码里所有字符串字面量（跳过整行注释与行尾注释）。
fn source_literals(src: &str) -> Vec<String> {
  let mut out = Vec::new();
  for line in src.lines() {
    if line.trim_start().starts_with("//") {
      continue;
    }
    let code = strip_trailing_comment(line);
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if bytes[i] == b'"' {
        match read_string(code, i) {
          Some((s, next)) => {
            out.push(s);
            i = next;
          }
          None => i += 1,
        }
      } else {
        i += 1;
      }
    }
  }
  out
}

/// 去掉行尾行内注释，避免把注释文字当字面量。
///
/// 逐字符扫描以跳过字符串字面量里的 `//`（如 `http://…`），否则这类含 `//` 的字符串
/// 会被误当作注释起点而截断。
fn strip_trailing_comment(line: &str) -> &str {
  let bytes = line.as_bytes();
  let mut in_string = false;
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => in_string = !in_string,
      b'\\' if in_string => i += 1, // 跳过转义字符，避免 `\"` 被当作字符串结束
      b'/' if !in_string && i + 1 < bytes.len() && bytes[i + 1] == b'/' => return &line[..i],
      _ => {}
    }
    i += 1;
  }
  line
}

/// 源码里用到、但某语言词典里没有的 key（按字典序，便于填模板）。
pub fn missing(root: &Path, lang: &str) -> Result<Vec<String>> {
  let known: HashSet<String> = load_entries(root, lang)?
    .into_iter()
    .map(|e| e.key)
    .collect();
  let mut used: Vec<String> = collect_call_sites(root)?
    .into_iter()
    .filter(|k| !known.contains(k))
    .collect();
  used.sort();
  Ok(used)
}

/// 词典里有、但已经用不上的 key。
///
/// 两种「还在用」都要认：源码里写死 `t("key")`，或该 key 的中文原文仍出现在源码里 ——
/// 导航分组名、页面标题这类文案定义在 `crates/core` 的 registry 中，是运行时传进 `t()`
/// 的（走 `catalog` 的中文反向索引），只比对 key 会把它们全误判成死条目。
///
/// 匹配用「字面量包含该中文」而不是「字面量等于」：文案常被 `format!` 拼进更长的句子，
/// 严格相等会大量误报。
pub fn unused(root: &Path, lang: &str) -> Result<Vec<String>> {
  let mut literals: HashSet<String> = HashSet::new();
  for base in ["crates/app/src", "crates/core/src"] {
    for f in walk(&root.join(base))? {
      // 与 [`collect_call_sites`] 同口径（先涂白注释与测试模块）：注释里举例的中文
      // 会把本该清理的死条目一直「救活」。
      literals.extend(source_literals(&read_source(&f)?));
    }
  }
  let used = collect_call_sites(root)?;
  let zh = zh_map(root)?;

  // 先用集合做 O(1) 精确筛选，只有没命中的候选才走 O(n) 的子串比对。
  let mut dead: Vec<String> = load_entries(root, lang)?
    .into_iter()
    .map(|e| e.key)
    .filter(|k| !used.contains(k))
    .filter(|k| !literals.contains(zh.get(k).map_or("", |v| v.as_str())))
    // 数据驱动的 key（`headers=&["knowledge.model", …]`）没有 `t("…")` 调用点，但 key 的
    // 字面量确实写在源码里 —— 那也是在用。不认这一条，这类表格页一加进来 `check-i18n` 就红。
    .filter(|k| !literals.contains(k))
    .collect();
  dead.sort();
  dead.dedup();
  dead.retain(|k| {
    // 中文词典里没有这个 key 时**不能**退化成空串：`l.contains("")` 恒真，会把死条目
    // 静默吞掉。三语 key 集由 `check-i18n` 要求一致，这里保守地按「确实死了」处理。
    let Some(text) = zh.get(k).map(String::as_str) else {
      return true;
    };
    !literals.iter().any(|l| l.contains(text))
  });
  Ok(dead)
}

/// 复数候选：源码按 `tf(` / `tp(` 调用、中文带数量占位符，而目标语言仍是普通文案。
///
/// 「调用」不限于语义 key：直接写中文原文的调用点会先经 [`ZhIndex`] 解析成语义 key，
/// 与写 key 的调用点合并 —— 两者在候选表里没有区别。
///
/// 只做「扫描 + 出骨架」，不自动改词典：`{}` 里到底哪个是 count、单复数怎么变形，
/// 机器判断不了（「共 {} 题」是数量，「{} 天内」未必），必须人工确认 —— 这正是
/// C3 不能全自动 codemod 的原因。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluralCandidate {
  pub key: String,
  /// 中文原文：占位符与语义的对照参考。
  pub zh: String,
  /// 该语言现有译文（骨架的 `other` 直接沿用它，填写时只需补 `one`）。
  pub current: String,
  /// 回填骨架 `{ "one": "", "other": <现译文> }`，与 `add-i18n` 的 `text` 字段同构。
  pub text: serde_json::Value,
  /// 调用点 `相对路径:行号`，供人工定位并确认 count 对应哪个占位符。
  pub calls: Vec<String>,
  /// 源码已改用 `tp()`：count 由作者确认过，优先处理（`tp` 却没配变体 = 白改）。
  pub uses_tp: bool,
}

/// 扫描复数候选（zh 不适用：中文不使用复数变体）。
pub fn plural_candidates(root: &Path, lang: &str) -> Result<Vec<PluralCandidate>> {
  let lang = check_lang(lang)?;
  if lang == "zh" {
    bail!("中文不使用复数变体，无需扫描复数候选");
  }
  Ok(candidates_of(
    &load_entries(root, lang)?,
    &zh_map(root)?,
    &call_locations(root)?,
    &collect_plural_call_sites(root)?,
  ))
}

/// 候选判定的纯逻辑部分（词典 × 中文 × 调用点 × `tp` 调用）。
fn candidates_of(
  entries: &[Entry],
  zh: &HashMap<String, String>,
  calls: &HashMap<String, Vec<String>>,
  tp_calls: &HashSet<String>,
) -> Vec<PluralCandidate> {
  let mut out = Vec::new();
  for e in entries {
    let Value::Plain(current) = &e.value else {
      continue; // 已迁成复数词条
    };
    // 只关心带参数调用的文案：`t("key")` 没有数量可依，谈不上复数。
    let Some(at) = calls.get(&e.key) else {
      continue;
    };
    let Some(text) = zh.get(&e.key) else {
      continue;
    };
    if count_placeholders(text) == 0 {
      continue; // 中文连占位符都没有，多半不是数量文案
    }
    out.push(PluralCandidate {
      key: e.key.clone(),
      zh: text.clone(),
      current: current.clone(),
      text: serde_json::json!({ "one": "", "other": current }),
      calls: at.clone(),
      uses_tp: tp_calls.contains(&e.key),
    });
  }
  // `tp` 已改过的排前面：它们已经确认过 count，只差词典里的变体。
  out.sort_by(|a, b| b.uses_tp.cmp(&a.uses_tp).then_with(|| a.key.cmp(&b.key)));
  out
}

/// 导出复数候选（写成 `{key, zh, text, calls, uses_tp}` 数组，可直接交给 `add-i18n`）。
///
/// `add-i18n` 只读 `key` 与 `text`，其余字段是给人看的；骨架里 `one` 留空，
/// 空值不会被合并，因此可以放心整份回填 —— 只填过的 `one` 会生效。
pub fn export_plural_candidates(root: &Path, lang: &str, out: &Path) -> Result<usize> {
  let items = plural_candidates(root, lang)?;
  if let Some(dir) = out.parent() {
    std::fs::create_dir_all(dir)?;
  }
  crate::fsutil::write_json(out, &items)?;
  Ok(items.len())
}

/// `tp()` 调用与词典复数变体的对账结果。
#[derive(Debug, Default)]
pub struct PluralReconciliation {
  /// 源码已用 `tp()` 调用、但词典里还是普通文案（当前会退化为 `other`，等于没换词形）。
  pub missing_variants: Vec<String>,
  /// 词典里已是复数词条、但源码没有 `tp()` 调用（变体配了也用不上）。
  pub unused_variants: Vec<String>,
}

/// 对账「`tp()` 调用 ↔ 复数变体」。
///
/// 两边都只是**提示**，不判失败：迁移是增量的（C3 分批做），硬报错会逼着一次改完。
pub fn reconcile_plural(root: &Path, lang: &str) -> Result<PluralReconciliation> {
  let lang = check_lang(lang)?;
  Ok(reconcile_of(
    &load_entries(root, lang)?,
    &collect_plural_call_sites(root)?,
  ))
}

/// 对账的纯逻辑部分（词典 × `tp` 调用）。
fn reconcile_of(entries: &[Entry], tp: &HashSet<String>) -> PluralReconciliation {
  let mut r = PluralReconciliation::default();
  let mut with_variants: HashSet<String> = HashSet::new();

  for e in entries {
    if matches!(e.value, Value::Plural(_)) {
      if !tp.contains(&e.key) {
        r.unused_variants.push(e.key.clone());
      }
      with_variants.insert(e.key.clone());
    }
  }
  for k in tp {
    if !with_variants.contains(k) {
      r.missing_variants.push(k.clone());
    }
  }
  r.missing_variants.sort();
  r.missing_variants.dedup();
  r.unused_variants.sort();
  r
}

/// 把已填写的模板合并回 `data/i18n/{lang}/{domain}.json`。
///
/// key 自带域前缀（`<domain>.<slug>`），目标文件因此是确定的 —— 不再需要按调用点猜域。
/// 已存在的普通文案跳过，空 text 也跳过 —— 因此可以对着同一份模板反复增量填写。
///
/// 唯一例外是**复数变体**：把一条普通文案升级成 `{ "one": …, "other": … }` 正是复数
/// 迁移的动作，因此复数对象允许覆盖已有的普通文案。
pub fn add(root: &Path, lang: &str, batch: &Path) -> Result<usize> {
  #[derive(serde::Deserialize)]
  struct Item {
    key: String,
    /// 字符串（普通文案）或对象（复数变体）。
    text: serde_json::Value,
  }
  let items: Vec<Item> = crate::fsutil::read_json(batch)?;
  let lang = check_lang(lang)?;
  let dir = lang_dir(root, lang);

  let mut by_domain: BTreeMap<String, EntryMap> = BTreeMap::new();
  for e in load_entries(root, lang)? {
    by_domain
      .entry(e.domain)
      .or_default()
      .insert(e.key, serde_json::to_value(&e.value)?);
  }
  let known: HashSet<String> = by_domain.values().flat_map(|m| m.keys().cloned()).collect();

  let mut n = 0;
  let mut bad = Vec::new();
  for it in items {
    let Ok(value) = serde_json::from_value::<Value>(it.text) else {
      bad.push(it.key);
      continue;
    };
    let upgrade = matches!(value, Value::Plural(_));
    if value.is_blank() || (!upgrade && known.contains(it.key.as_str())) {
      continue;
    }
    let Some(domain) = it.key.split('.').next().filter(|d| !d.is_empty()) else {
      bad.push(it.key);
      continue;
    };
    // `domain` 会被直接拼进路径（`data/i18n/{lang}/{domain}.json`），而批次文件里
    // 的 key 是外部输入：不校验的话 `a/../../x.y` 这样的 key 能把文件写到项目外
    // （`write_json_if_changed` 会顺带 `create_dir_all`）。域名只可能是小写字母、
    // 数字与连字符，其余一律按「key 不合法」跳过。
    if !domain
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
      bad.push(it.key);
      continue;
    }
    by_domain
      .entry(domain.to_owned())
      .or_default()
      .insert(it.key, serde_json::to_value(&value)?);
    n += 1;
  }
  if n == 0 {
    return Ok(0);
  }

  for (domain, map) in &by_domain {
    // 只写内容真的变了的域：回填常常只涉及一两个域，其余文件不该被重写一遍。
    crate::fsutil::write_json_if_changed(&dir.join(format!("{domain}.json")), map)?;
  }
  if !bad.is_empty() {
    println!("（{} 条 key 缺少域前缀，已跳过：{}…）", bad.len(), bad[0]);
  }
  Ok(n)
}

/// 运行时语言包的目录：`public/data/i18n/{lang}.json`。
///
/// 放在 `public/` 而不是 `dist/`：Trunk 在 `trunk serve`（开发）与 `trunk build`（发布）
/// 都会把 `public/` 拷进产物目录，开发与发布因此共用同一份文件，`sw.js` 的预缓存清单
/// 也会自动把它收进去 —— 包内容一变，预缓存清单的内容哈希就变，Service Worker 版本号
/// 随之变化，离线缓存自动刷新，不需要额外的 `?v=` 参数。
fn pack_dir(root: &Path) -> PathBuf {
  root.join("public").join("data").join("i18n")
}

/// 一种语言的语言包词条（全部域合并，按 key 排序 —— 输出稳定可复现）。
///
/// 含内嵌域（`common` / `shell`）：那几十 KB 换来「不必在构建脚本与这里各维护一份
/// 内嵌域清单」，前端查内嵌表优先，结果一致。
///
/// 复数词条单独成一段（`plural`）：前端按 CLDR 类别取变体，扁平表里不再出现这些 key。
fn pack_parts(root: &Path, lang: &str) -> Result<(FlatPack, PluralPack)> {
  let mut flat = BTreeMap::new();
  let mut plural = BTreeMap::new();
  for e in load_entries(root, lang)? {
    match e.value {
      Value::Plain(v) => {
        flat.insert(e.key, v);
      }
      Value::Plural(v) => {
        plural.insert(e.key, v);
      }
    }
  }
  Ok((flat, plural))
}

/// 语言包的扁平段：`key → 文本`。
type FlatPack = BTreeMap<String, String>;
/// 语言包的复数段：`key → (类别 → 文本)`。
type PluralPack = BTreeMap<String, BTreeMap<String, String>>;

/// 语言包的 JSON 结构：扁平词条 + 复数词条。
#[derive(serde::Serialize)]
struct PackPayload<'a> {
  flat: &'a FlatPack,
  plural: &'a PluralPack,
}

/// 语言包的字节内容（紧凑 JSON，不缩进 —— 这是要下载的产物）。
fn pack_bytes(flat: &FlatPack, plural: &PluralPack) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec(&PackPayload { flat, plural })?;
  bytes.push(b'\n');
  Ok(bytes)
}

/// 生成运行时语言包，返回写入的词条数（en + es）。
///
/// 中文不产出：中文全量编进 wasm（`crates/app/build.rs` 只对非中文语言裁域），
/// 默认语言不该为「界面本来就是中文」付一次网络请求。
pub fn write_pack(root: &Path) -> Result<usize> {
  let dir = pack_dir(root);
  std::fs::create_dir_all(&dir)?;
  let mut total = 0;
  for lang in ["en", "es"] {
    let (flat, plural) = pack_parts(root, lang)?;
    let n = flat.len() + plural.len();
    std::fs::write(
      dir.join(format!("{lang}.json")),
      pack_bytes(&flat, &plural)?,
    )?;
    println!("Generated public/data/i18n/{lang}.json ({n} entries)");
    total += n;
  }
  Ok(total)
}

/// 产物是否已过期：`data/i18n/` 改了却没重新生成语言包。
///
/// 过期的后果是「译文改了但线上没变」，且不会有任何报错 —— 因此由 `check-i18n` 兜住。
pub fn pack_is_stale(root: &Path) -> bool {
  ["en", "es"].iter().any(|lang| {
    // 出错时**当作陈旧**：词典坏掉（JSON 语法错、结构不对）时若返回「不陈旧」，
    // 校验就在异常路径上静默放行了 —— 而那恰恰是最该报错的情况。
    let Ok((flat, plural)) = pack_parts(root, lang) else {
      return true;
    };
    let Ok(bytes) = pack_bytes(&flat, &plural) else {
      return true;
    };
    match std::fs::read(pack_dir(root).join(format!("{lang}.json"))) {
      Ok(on_disk) => on_disk != bytes,
      Err(_) => true, // 还没生成过
    }
  })
}

/// 载入 `data/i18n/` 并校验 zh / en / es 三个语言。
///
/// 同时统计「源码里用了但词典里没有」与「词典里有但没人用」—— 前者说明翻译没跟上，
/// 后者说明文案删了译文没清。
pub fn run(root: &Path) -> Result<bool> {
  let mut ok = true;
  let mut per_domain: BTreeMap<String, usize> = BTreeMap::new();
  let zh = zh_map(root)?;

  for lang in LANGS {
    let label = match lang {
      "zh" => "中文",
      "en" => "英文",
      _ => "西班牙文",
    };
    let entries = load_entries(root, lang)?;
    let r = check(&entries, &zh, lang);
    for e in &entries {
      *per_domain.entry(e.domain.clone()).or_insert(0) += 1;
    }
    println!("{label}（{lang}）：{} 条词条", entries.len());
    if !r.duplicates.is_empty() {
      ok = false;
      println!("  ✗ 重复 key {} 条：", r.duplicates.len());
      for k in r.duplicates.iter().take(10) {
        println!("      {k}");
      }
    }
    if !r.bad_namespace.is_empty() {
      ok = false;
      println!("  ✗ key 与所在域不一致 {} 条：", r.bad_namespace.len());
      for k in r.bad_namespace.iter().take(10) {
        println!("      {k}");
      }
    }
    if !r.placeholder_mismatch.is_empty() {
      ok = false;
      println!(
        "  ✗ 占位符数量与中文不一致 {} 条：",
        r.placeholder_mismatch.len()
      );
      for (k, base, tr) in r.placeholder_mismatch.iter().take(10) {
        println!("      {k}：中文 {base} 个 / 译文 {tr} 个");
      }
    }
    if !r.bad_plural.is_empty() {
      ok = false;
      println!("  ✗ 复数变体有问题 {} 条：", r.bad_plural.len());
      for k in r.bad_plural.iter().take(10) {
        println!("      {k}");
      }
    }
    // 守门：存量已清零，新增的多计数文案在这里就被拦下（按 P3-C5 的 L2 / L3 改写）。
    if !r.multi_count.is_empty() {
      ok = false;
      println!("  ✗ 一句话多个可数名词 {} 条：", r.multi_count.len());
      for k in r.multi_count.iter().take(10) {
        println!("      {k}");
      }
    }
    if r.duplicates.is_empty()
      && r.bad_namespace.is_empty()
      && r.placeholder_mismatch.is_empty()
      && r.bad_plural.is_empty()
      && r.multi_count.is_empty()
    {
      println!("  ✓ 无重复 / 命名空间一致 / 占位符一致 / 复数变体合法 / 无多计数文案");
    }
  }

  println!("\n按域词条数（zh + en + es）：");
  let mut rows: Vec<_> = per_domain.iter().collect();
  rows.sort_by(|a, b| b.1.cmp(a.1));
  for (d, n) in rows {
    println!("  {n:>6}  {d}");
  }

  // 源码里用到的 key 是否都在词典里（写错 key 会在这里现形）
  match collect_call_sites(root) {
    Ok(used) => {
      for lang in LANGS {
        let known: HashSet<String> = load_entries(root, lang)?
          .into_iter()
          .map(|e| e.key)
          .collect();
        let miss: Vec<&String> = used.iter().filter(|k| !known.contains(*k)).collect();
        let done = used.len() - miss.len();
        let pct = if used.is_empty() {
          100.0
        } else {
          done as f64 * 100.0 / used.len() as f64
        };
        println!(
          "\n{lang} 覆盖率：{done} / {}（{pct:.1}%），缺 {} 条",
          used.len(),
          miss.len()
        );
        for k in miss.iter().take(5) {
          println!("      待补：{k}");
        }
        if miss.len() > 5 {
          println!("      … 另有 {} 条", miss.len() - 5);
        }
        // 硬失败：`t()` 查不到 key 时会**原样返回 key**（见 `i18n::zh_resolved`），
        // 所以「漏一条」的线上表现是界面直接漏出 `shell.hme` —— 不能只提示。
        if !miss.is_empty() {
          ok = false;
        }
      }
    }
    Err(e) => {
      // 统计不出来就不能声称「守住了」：宁可红一次，也别让漏 key 静默通过。
      ok = false;
      println!("\n✗ 覆盖率统计失败：{e:#}");
    }
  }

  // 死条目：词典里有、但已经用不上了
  match unused(root, "zh") {
    Ok(dead) if !dead.is_empty() => {
      ok = false;
      println!("\n死条目 {} 条（源码里已用不上，请清理）：", dead.len());
      for k in dead.iter().take(50) {
        println!("      {k}");
      }
      if dead.len() > 50 {
        println!("      … 另有 {} 条", dead.len() - 50);
      }
    }
    Ok(_) => println!("\n✓ 无死条目"),
    Err(e) => {
      ok = false;
      println!("\n✗ 死条目统计失败：{e:#}");
    }
  }

  // `tp()` 调用 ↔ 词典复数变体对账（只提示、不判失败：迁移是分批做的）
  for lang in ["en", "es"] {
    match reconcile_plural(root, lang) {
      Ok(r) => {
        if !r.missing_variants.is_empty() {
          println!(
            "\n! {lang}：{} 条文案已改用 tp()，但词典还没配复数变体（当前退化为 other）",
            r.missing_variants.len()
          );
          for k in r.missing_variants.iter().take(10) {
            println!("      {k}");
          }
          if r.missing_variants.len() > 10 {
            println!("      … 另有 {} 条", r.missing_variants.len() - 10);
          }
        }
        if !r.unused_variants.is_empty() {
          println!(
            "\n! {lang}：{} 条文案已配复数变体，但源码没有 tp() 调用（变体暂时用不上）",
            r.unused_variants.len()
          );
          for k in r.unused_variants.iter().take(10) {
            println!("      {k}");
          }
          if r.unused_variants.len() > 10 {
            println!("      … 另有 {} 条", r.unused_variants.len() - 10);
          }
        }
      }
      Err(e) => println!("\n（跳过 {lang} 复数对账：{e:#}）"),
    }
  }
  match plural_candidates(root, "en") {
    Ok(c) if !c.is_empty() => println!(
      "\n复数候选 {} 条（tf/tp 调用且中文带数量占位符）：\
       cargo make i18n-plural-candidates 导出骨架",
      c.len()
    ),
    Ok(_) => {}
    Err(e) => println!("\n（跳过复数候选统计：{e:#}）"),
  }

  // 运行时语言包是否跟着 `data/i18n/` 刷新过
  if pack_is_stale(root) {
    ok = false;
    println!("\n✗ 运行时语言包已过期：请执行 cargo make i18n-pack");
  } else {
    println!("\n✓ 运行时语言包 public/data/i18n/ 与 data/i18n/ 一致");
  }

  // 三种语言的 key 集合必须完全一致：[`check`] 只做单语言的重复 / 命名空间检查，
  // `catalog.rs` 的测试只管「en/es ⊆ zh」，非内嵌域少一条谁都发现不了 —— 而少的那条
  // 在运行时的表现是「非中文回退中文，甚至直接漏出 key」。
  let mut key_sets: Vec<(&str, HashSet<String>)> = Vec::new();
  for lang in LANGS {
    let keys = load_entries(root, lang)?
      .into_iter()
      .map(|e| e.key)
      .collect::<HashSet<String>>();
    key_sets.push((lang, keys));
  }
  let union: HashSet<&str> = key_sets
    .iter()
    .flat_map(|(_, s)| s.iter().map(String::as_str))
    .collect();
  let mut parity: Vec<String> = Vec::new();
  for (lang, keys) in &key_sets {
    for k in &union {
      if !keys.contains(*k) {
        parity.push(format!("{lang} 缺 {k}"));
      }
    }
  }
  if parity.is_empty() {
    println!(
      "\n✓ zh / en / es 的 key 集合一致（各 {} 条）",
      key_sets[0].1.len()
    );
  } else {
    ok = false;
    parity.sort();
    println!("\n✗ 三种语言的 key 集合不一致 {} 处：", parity.len());
    for p in parity.iter().take(10) {
      println!("      {p}");
    }
    if parity.len() > 10 {
      println!("      … 另有 {} 处", parity.len() - 10);
    }
  }

  // 中文释义必须唯一：`catalog::zh_reverse()` 用「中文原文 → key」建反查表
  // （`crates/core` 的 registry 标题走这条路），重复释义会让其中一条永远命中不了 ——
  // 而且是静默的：谁都没报错，只是换了语言显示的是另一条 key 的译文。
  let mut by_text: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
  for (k, v) in &zh {
    by_text.entry(v.as_str()).or_default().push(k.as_str());
  }
  let mut dup_text: Vec<String> = by_text
    .iter()
    .filter(|(_, ks)| ks.len() > 1)
    .map(|(text, ks)| format!("{text}：{}", ks.join(" / ")))
    .collect();
  if dup_text.is_empty() {
    println!("\n✓ 中文释义唯一（{} 条，无重复）", zh.len());
  } else {
    ok = false;
    dup_text.sort();
    println!(
      "\n✗ 中文释义重复 {} 组（反向索引只能命中其中一条，另一条等于查不到）：",
      dup_text.len()
    );
    for d in dup_text.iter().take(10) {
      println!("      {d}");
    }
  }

  // `tf` / `tp` 的实参里写中文（`t()` 的入参走反向索引，受支持；实参不走，会直接漏出）
  match cjk_args(root) {
    Ok(hits) if hits.is_empty() => println!("\n✓ tf / tp 的实参里没有硬编码中文"),
    Ok(hits) => {
      ok = false;
      println!(
        "\n✗ tf / tp 的实参里出现中文 {} 处（en / es 界面会原样漏出）：",
        hits.len()
      );
      for h in hits.iter().take(10) {
        println!("      {h}");
      }
    }
    Err(e) => {
      ok = false;
      println!("\n✗ 实参中文扫描失败：{e:#}");
    }
  }

  // 编号 slug 守门：存量冻结在 [`NUMBERED_SLUG_BASELINE`]，新增一律失败 ——
  // 别让第 151 条 `xxx-2` 再长出来。
  let mut numbered: BTreeSet<String> = BTreeSet::new();
  for (_, keys) in &key_sets {
    for k in keys {
      if is_collision_slug(k, keys) {
        numbered.insert(k.clone());
      }
    }
  }
  let fresh: Vec<&str> = numbered
    .iter()
    .map(String::as_str)
    .filter(|k| !NUMBERED_SLUG_BASELINE.contains(k))
    .collect();
  if fresh.is_empty() {
    println!(
      "\n✓ 无新增编号 slug（存量 {} 条待重命名，清单见 NUMBERED_SLUG_BASELINE）",
      numbered.len()
    );
  } else {
    ok = false;
    println!(
      "\n✗ 新增了 {} 条编号 slug（slug 冲突自动加序号，语义为零）：",
      fresh.len()
    );
    for k in fresh.iter().take(10) {
      println!("      {k}");
    }
  }
  let stale = NUMBERED_SLUG_BASELINE
    .iter()
    .filter(|k| !numbered.contains(**k))
    .count();
  if stale > 0 {
    println!("      （基线里另有 {stale} 条已不存在，可以从清单里删掉）");
  }

  // 静态扫描认不出来的调用点：只提示，不判失败（工具无法自动解析）。
  match unresolved_calls(root) {
    Ok(v) if v.is_empty() => println!("\n✓ 全部文案调用点都是字面量 key"),
    Ok(v) => {
      println!(
        "\n! {} 处调用点的首个实参不是字面量（`t(&format!(…))` / 变量传入）：\
         静态扫描认不出来，对应的 key 可能被误判成死条目或漏出覆盖率，请人工核对：",
        v.len()
      );
      for x in v.iter().take(10) {
        println!("      {x}");
      }
      if v.len() > 10 {
        println!("      … 另有 {} 处", v.len() - 10);
      }
    }
    Err(e) => println!("\n（跳过动态调用点统计：{e:#}）"),
  }

  Ok(ok)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(k: &str, v: &str, d: &str) -> Entry {
    Entry {
      key: k.to_owned(),
      value: Value::Plain(v.to_owned()),
      domain: d.to_owned(),
    }
  }

  /// 复数词条：`variants` 形如 `[("one", "{} day"), ("other", "{} days")]`。
  fn plural_entry(k: &str, variants: &[(&str, &str)], d: &str) -> Entry {
    Entry {
      key: k.to_owned(),
      value: Value::Plural(
        variants
          .iter()
          .map(|(c, v)| ((*c).to_owned(), (*v).to_owned()))
          .collect(),
      ),
      domain: d.to_owned(),
    }
  }

  fn zh_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
      .iter()
      .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
      .collect()
  }

  #[test]
  fn detects_duplicate_namespace_and_placeholder_mismatch() {
    let zh = zh_of(&[("a.one", "共 {} 题"), ("a.two", "答对 {} / {}")]);
    let r = check(
      &[
        entry("a.one", "{} questions", "a"),
        entry("a.one", "{} items", "a"), // 重复 key
        entry("a.two", "{} right", "a"), // 少一个占位符
        entry("b.one", "x", "a"),        // key 前缀与域不一致
      ],
      &zh,
      "en",
    );
    assert_eq!(r.duplicates.len(), 1, "{:?}", r.duplicates);
    assert_eq!(r.bad_namespace, vec!["b.one"]);
    assert_eq!(
      r.placeholder_mismatch.len(),
      1,
      "{:?}",
      r.placeholder_mismatch
    );
    // 中文 2 个 / 译文 1 个
    assert_eq!(r.placeholder_mismatch[0], ("a.two".to_owned(), 2, 1));
    assert!(r.bad_plural.is_empty());
  }

  /// 复数变体：每个变体单独比对中文侧的占位符，且必须含 `other`。
  #[test]
  fn checks_plural_variants() {
    let zh = zh_of(&[("a.days", "{} 天")]);
    let r = check(
      &[
        plural_entry("a.days", &[("one", "{} day"), ("other", "{} days")], "a"),
        plural_entry("a.days", &[("one", "{} day")], "a"), // 缺 other
      ],
      &zh,
      "en",
    );
    assert_eq!(r.bad_plural.len(), 1, "{:?}", r.bad_plural);
    assert!(r.bad_plural[0].contains("a.days"));
    assert!(
      r.placeholder_mismatch.is_empty(),
      "{:?}",
      r.placeholder_mismatch
    );
  }

  #[test]
  fn rejects_unknown_plural_category_and_mismatched_variant() {
    let zh = zh_of(&[("a.days", "{} 天")]);
    let r = check(
      &[plural_entry(
        "a.days",
        &[("some", "{} day"), ("other", "days")],
        "a",
      )],
      &zh,
      "en",
    );
    assert_eq!(r.bad_plural.len(), 1, "{:?}", r.bad_plural);
    assert!(r.bad_plural[0].contains("some"));
    // `other` 少一个占位符
    assert_eq!(
      r.placeholder_mismatch,
      vec![("a.days#other".to_owned(), 1, 0)]
    );
  }

  /// 中文保持扁平：复数变体是 en / es 的事。
  #[test]
  fn chinese_dictionary_rejects_plural_variants() {
    let zh = zh_of(&[("a.days", "{} 天")]);
    let r = check(
      &[plural_entry("a.days", &[("other", "{} 天")], "a")],
      &zh,
      "zh",
    );
    assert_eq!(r.bad_plural.len(), 1, "{:?}", r.bad_plural);
  }

  /// 一句话里 ≥ 2 处「数量 + 可数名词」要报出来：`tp()` 只吃一个 count，管不了。
  ///
  /// 词表自维护（从复数变体派生）+ 兜底名单，且只有会变形的词才算数 ——
  /// 否则 `{} % correct / {} answered` 这类没复数形式的词会天天误报。
  #[test]
  fn flags_sentences_with_multiple_countable_nouns() {
    let zh = zh_of(&[("a.days", "{} 天")]);
    let r = check(
      &[
        plural_entry("a.days", &[("one", "{} day"), ("other", "{} days")], "a"),
        entry("b.many", "{} days · {} days", "b"), // 2 处 → 报
        entry("b.one", "{} day", "b"),             // 1 处 → 不报
        entry("b.extra", "{} QSOs · {} pages", "b"), // 兜底名单也认 → 报
        entry("b.noise", "{} correct · {} answered", "b"), // 无复数形式 → 不报
      ],
      &zh,
      "en",
    );
    let flagged: Vec<String> = r
      .multi_count
      .iter()
      .map(|s| s.split('：').next().unwrap_or("").to_owned())
      .collect();
    assert_eq!(flagged, vec!["b.many", "b.extra"]);
  }

  /// `tp("…", n, …)` 与 `t("…")` / `tf("…")` 一样是「这个 key 还在用」的证据，
  /// 同时 `tp` 本身要能被单独挑出来 —— 复数候选扫描与对账都靠它。
  ///
  /// `set_title("…")` 存的是 key（在 `apply_title` 里才查表），必须一起认；但它也接受
  /// 裸标题（`APRS`），那种不该进覆盖率 —— 否则会报出三条不存在的「待补」。
  #[test]
  fn collect_call_sites_recognizes_tp_and_set_title() {
    let src = r#"let a = tf("a.b", &[x]); let c = tp("c.d", n, &[y]); s.set_title("e.f"); s.set_title("APRS"); s.set_title("异常页面");"#;
    let mut keys: Vec<String> = scan_calls(src).into_iter().map(|(_, _, k)| k).collect();
    keys.sort();
    assert_eq!(keys, vec!["a.b", "c.d", "e.f"]);
    let mut kinds: Vec<&str> = scan_calls(src).into_iter().map(|(_, k, _)| k).collect();
    kinds.sort();
    assert_eq!(kinds, vec!["set_title", "tf", "tp"]);
  }

  /// `set_title_with_args("radio.print", &[…])` 同样存语义 key，也必须认；
  /// 长名字要排在 `set_title(` 之前判断，否则会被误当成后者而漏掉（前缀相同）。
  #[test]
  fn scan_calls_recognizes_set_title_with_args() {
    let src = r#"set_title_with_args("radio.print", &[x]); set_title("e.f");"#;
    let calls = scan_calls(src);
    let mut kinds: Vec<&str> = calls.iter().map(|(_, k, _)| *k).collect();
    kinds.sort();
    assert_eq!(kinds, vec!["set_title", "set_title_with_args"]);
    let mut keys: Vec<&str> = calls.iter().map(|(_, _, k)| k.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["e.f", "radio.print"]);
  }

  /// `first_arg_end` 必须把**第一个实参**（key 的位置）与后续实参分开。
  ///
  /// 守门规则（[`cjk_args`]）判定「是不是 key」不能数字面量序号：`tf(label, "中文")`
  /// 的首参不是字面量，`literals_at` 返回的第一个字面量其实是那个中文实参 —— 用
  /// `n == 0` 跳过它，就正好把该报的漏掉了。
  #[test]
  fn first_arg_end_separates_the_key_from_the_arguments() {
    let region = |src: &str| {
      let open = src.find('(').unwrap();
      let end = src.rfind(')').unwrap();
      (open, end, first_arg_end(src, open, end))
    };

    // 首参非字面量：中文实参落在 key 区间之后，必须算进「实参里的中文」。
    let src = r#"tf(label, "中文")"#;
    let (open, end, key_end) = region(src);
    assert_eq!(src[open + 1..key_end].trim(), "label");
    let lits = literals_at(src, open, end);
    assert_eq!(lits.len(), 1);
    assert!(lits[0].1 > key_end, "中文实参必须落在 key 区间之后");

    // 首参是字面量：它自己就是 key（允许中文原文），不该被当成漏翻译；
    // 第二个字面量才是实参，仍要被认出来（所以只跳过首参那一个）。
    let src = r#"tf("中文 key", "参数")"#;
    let (open, end, key_end) = region(src);
    let lits = literals_at(src, open, end);
    assert_eq!(lits.len(), 2);
    assert!(lits[0].1 < key_end, "第一个字面量是 key");
    assert!(
      lits[1].1 > key_end,
      "第二个字面量是真参，必须落在 key 区间之后"
    );

    // 首参是表达式（`if … { … } else { … }`）：里面的字面量仍属 key 位置。
    let src = r#"tp(if ok { "甲" } else { "乙" }, n, &args)"#;
    let (open, end, key_end) = region(src);
    for (lit, at) in literals_at(src, open, end) {
      assert!(at < key_end, "首参里的字面量 {lit} 不该被当成实参");
    }

    // 字符串里带逗号 / 括号不能被当成参数分隔符。
    let src = r#"tf(k, "a,b(c)", d)"#;
    let (_, _, key_end) = region(src);
    assert_eq!(key_end, src.find(',').unwrap(), "顶层逗号才是首参的边界");
  }

  /// 偏移量用来算行号：候选报告要给出 `文件:行号` 供人工确认 count 对应哪个占位符。
  #[test]
  fn scan_calls_reports_line_numbers() {
    let src = "let a = t(\"a.b\");\nlet c = tp(\n  \"c.d\",\n  n,\n  &[y],\n);\n";
    let calls = scan_calls(src);
    let (offset, kind, key) = calls
      .iter()
      .find(|(_, k, _)| *k == "tp")
      .cloned()
      .expect("跨行调用也要能扫到");
    assert_eq!(kind, "tp");
    assert_eq!(key, "c.d");
    // 偏移量换算出的行号指向调用起点（`tp(` 所在行）
    assert_eq!(src[..offset].matches('\n').count() + 1, 2);
    // 跨行调用（`(` 后换行）不能被漏掉
    assert_eq!(calls.len(), 2);
  }

  /// 复数候选：只收「带参数调用 + 中文有占位符 + 词典还是普通文案」的词条，
  /// 并给出 `one` 留空、`other` 沿用现译文的骨架。
  #[test]
  fn plural_candidates_need_a_count_placeholder() {
    let zh = zh_of(&[
      ("a.days", "{} 天"),     // 候选
      ("a.total", "共 {} 题"), // 候选（已改 tp）
      ("a.plain", "无占位符"), // 不是候选：没有数量
      ("a.only_t", "{} 次"),   // 不是候选：只用 t() 调用
    ]);
    let calls: HashMap<String, Vec<String>> = HashMap::from([
      ("a.days".to_owned(), vec!["src/a.rs:1".to_owned()]),
      ("a.total".to_owned(), vec!["src/a.rs:2".to_owned()]),
      ("a.plain".to_owned(), vec!["src/a.rs:3".to_owned()]),
    ]);
    let tp = HashSet::from(["a.total".to_owned()]);
    let out = candidates_of(
      &[
        entry("a.days", "{} days", "a"),
        entry("a.total", "{} questions", "a"),
        entry("a.plain", "plain", "a"),
        entry("a.only_t", "{} times", "a"),
        // 已迁成复数词条：不再出现在候选里
        plural_entry("a.migrated", &[("one", "1"), ("other", "n")], "a"),
      ],
      &zh,
      &calls,
      &tp,
    );

    // `a.total` 用了 tp，排在前面
    let keys: Vec<&str> = out.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, vec!["a.total", "a.days"]);
    assert!(out[0].uses_tp);
    assert!(!out[1].uses_tp);
    assert_eq!(out[1].zh, "{} 天");
    // 骨架：one 待填、other 沿用现译文（人工只需补 one）
    assert_eq!(
      out[1].text,
      serde_json::json!({ "one": "", "other": "{} days" })
    );
    assert_eq!(out[1].calls, vec!["src/a.rs:1"]);
  }

  /// 对账：`tp()` 调用但没配变体 / 配了变体但没有 `tp()` 调用，两边都能报出来。
  #[test]
  fn reconciles_tp_calls_with_plural_variants() {
    let r = reconcile_of(
      &[
        entry("a.days", "{} days", "a"),              // tp 调用，但没配变体
        plural_entry("a.q", &[("other", "n")], "a"),  // 配了变体，但没用 tp
        plural_entry("a.ok", &[("other", "n")], "a"), // 两边都对得上
      ],
      &HashSet::from(["a.days".to_owned(), "a.ok".to_owned()]),
    );
    assert_eq!(r.missing_variants, vec!["a.days"]);
    assert_eq!(r.unused_variants, vec!["a.q"]);
  }

  /// 测试模块先涂白再扫：单测里构造的 `tp()` 不是真实用法（会让复数对账误报）。
  #[test]
  fn masks_test_modules_before_scanning() {
    let src = concat!(
      "let a = t(\"a.b\");\n",
      "#[cfg(test)]\n",
      "mod tests {\n",
      "  fn x() { tp(\"c.d\", 1, &[]); }\n",
      "}\n",
      "let e = t(\"e.f\");\n",
    );
    assert_eq!(keys_of(&mask_test_modules(src)), vec!["a.b", "e.f"]);
    // 涂白而非删除：行数与字节偏移都不变，报告里的行号才准
    assert_eq!(mask_test_modules(src).lines().count(), src.lines().count());
  }

  #[test]
  fn counts_placeholders() {
    assert_eq!(count_placeholders("a {} b {} c"), 2);
    assert_eq!(count_placeholders("无占位符"), 0);
  }

  #[test]
  fn rejects_unknown_lang() {
    assert!(check_lang("fr").is_err());
    assert!(check_lang("zh").is_ok());
  }

  /// 回归：`t(` 后不是引号时 `read_string` 返回 None，若用 `continue` 会跳过
  /// `i += 1` 导致死循环（此测试一旦回归会直接超时，而非失败）。
  #[test]
  fn scan_calls_survives_malformed_call() {
    let src = "let a = t(1); let b = t(\"a.b\");";
    assert_eq!(keys_of(src), vec!["a.b"]);
  }

  /// `split("…")` / `insert("…")` 以 `t` 结尾，不能被当成 `t()` 文案调用。
  #[test]
  fn scan_calls_ignores_identifiers_ending_in_t() {
    let src = r#"let a = s.split("x"); let b = t("a.b");"#;
    assert_eq!(keys_of(src), vec!["a.b"]);
  }

  /// 调用点直接写中文原文时（`tf("已合并 {} 条数据", …)`），也要能被反查成语义 key：
  /// 只认语义 key 的话这类调用点永远进不了复数候选表，成了静态扫描的盲区。
  #[test]
  fn resolves_chinese_literal_calls_to_semantic_keys() {
    let index = ZhIndex::from_zh(zh_of(&[
      ("a.days", "{} 天"),
      ("a.merged", "已合并 {} 条数据"),
      ("a.plain", "无占位符"),
    ]));
    // 语义 key 原样返回
    assert_eq!(index.resolve("a.days"), "a.days");
    // 整条中文原文 → 语义 key
    assert_eq!(index.resolve("已合并 {} 条数据"), "a.merged");
    // 原文被拼进更长的句子：按最长原文匹配，不会被「{} 天」之类抢走
    assert_eq!(index.resolve("已合并 {} 条数据，请刷新页面"), "a.merged");
    // 没有占位符的原文不参与包含匹配，否则「条」「次」这类短原文会命中一切
    assert_eq!(index.resolve("无占位符"), "a.plain");
    assert_eq!(index.resolve("前缀 无占位符 后缀"), "前缀 无占位符 后缀");
  }

  /// 只取 key，省得每个断言都写一遍元组拆解。
  fn keys_of(src: &str) -> Vec<String> {
    scan_calls(src).into_iter().map(|(_, _, k)| k).collect()
  }
}
