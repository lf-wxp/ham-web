//! 题目解析维护：合并、术语增强、缺失统计、重新应用。
//!
//! 解析统一存放在 `data/explanations.json`，key 为题目内容指纹（见 `ham_web_core::fingerprint`），
//! value 为解析文本。指纹与题号/顺序/题库无关，因此题库重排不会导致解析错位，
//! A/B/C 中内容相同的题目共享同一条解析。

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use ham_web_core::glossary::Glossary;
use ham_web_core::text::{is_js_whitespace, js_trim};
use ham_web_core::typography::normalize;
use ham_web_core::{Bank, QuestionItem, content_key, fingerprint};
use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::fsutil::{Paths, read_json, write_json};

/// 读取解析表，兼容对象（`{指纹: 文本}`）与数组（`[{id, text}]`）两种格式；读取失败返回空表。
pub fn load_explanations(path: &Path) -> HashMap<String, String> {
  if !path.is_file() {
    return HashMap::new();
  }
  match read_json::<Value>(path) {
    Ok(Value::Object(obj)) => obj
      .into_iter()
      .map(|(k, v)| {
        (
          js_trim(&k).to_owned(),
          js_trim(v.as_str().unwrap_or_default()).to_owned(),
        )
      })
      .collect(),
    Ok(Value::Array(items)) => items
      .into_iter()
      .filter_map(|it| {
        let id = it.get("id")?.as_str()?;
        let text = it.get("text").and_then(Value::as_str).unwrap_or_default();
        Some((js_trim(id).to_owned(), js_trim(text).to_owned()))
      })
      .collect(),
    Ok(_) => HashMap::new(),
    Err(e) => {
      eprintln!("Failed to load explanations.json: {e:#}");
      HashMap::new()
    }
  }
}

/// 读取保持顺序的解析表（用于写回）。
fn load_ordered(path: &Path) -> Result<Map<String, Value>> {
  if !path.is_file() {
    return Ok(Map::new());
  }
  read_json(path)
}

/// 题库名 → 题目列表。`full` 是完整题库，同样需要解析，因此一并纳入维护。
fn load_banks(paths: &Paths) -> Result<Vec<(String, Vec<QuestionItem>)>> {
  ["A", "B", "C", "full"]
    .into_iter()
    .map(|name| Ok((name.to_owned(), read_json(&paths.bank_json(name))?)))
    .collect::<Result<Vec<_>>>()
    .context("题库 JSON 不存在或损坏，请先运行 `cargo make dataset`")
}

fn has_explanation(q: &QuestionItem) -> bool {
  q.explanation
    .as_deref()
    .is_some_and(|s| !js_trim(s).is_empty())
}

/// 按题目 ID 合并解析。
pub fn add(paths: &Paths, batch_path: &Path) -> Result<()> {
  let batch: Map<String, Value> = read_json(batch_path)?;
  let banks = load_banks(paths)?;
  let by_id: HashMap<&str, &QuestionItem> = banks
    .iter()
    .flat_map(|(_, qs)| qs.iter())
    .filter_map(|q| q.id_str().map(|id| (id, q)))
    .collect();
  let mut explanations = load_ordered(&paths.explanations)?;

  let (mut added, mut unknown, mut empty) = (0usize, 0usize, 0usize);
  for (id, text) in &batch {
    let Some(q) = by_id.get(id.as_str()) else {
      eprintln!("未知 id: {id}");
      unknown += 1;
      continue;
    };
    let text = match text {
      Value::String(s) => normalize(js_trim(s)),
      other => normalize(js_trim(&other.to_string())),
    };
    if text.is_empty() {
      empty += 1;
      continue;
    }
    explanations.insert(fingerprint(q), Value::String(text));
    added += 1;
  }

  write_json(&paths.explanations, &explanations)?;
  println!("已写入 {added} 条解析；未知 id {unknown} 个；跳过空解析 {empty} 条");
  println!("当前 explanations.json 共 {} 条", explanations.len());
  println!("提示：运行 `cargo make explanations-apply` 把解析写入题库 JSON");
  Ok(())
}

/// 术语后紧跟这些字符时，视为已解释/处于括号结构内，跳过注入。
const OPENERS: [char; 4] = ['（', '(', '）', ')'];

fn find_all(haystack: &[char], needle: &[char]) -> Vec<usize> {
  let mut out = Vec::new();
  if needle.is_empty() || needle.len() > haystack.len() {
    return out;
  }
  let mut from = 0;
  while from + needle.len() <= haystack.len() {
    match haystack[from..]
      .windows(needle.len())
      .position(|w| w == needle)
    {
      Some(off) => {
        let p = from + off;
        out.push(p);
        from = p + needle.len();
      }
      None => break,
    }
  }
  out
}

/// 注入时视为「更长词语」而整体跳过的复合词。
///
/// 这些词内部含有术语表中的短术语（视距 / 载波 / 调频 / 音频 / 带宽 / 极化 …），
/// 但组合后的语义已经改变：把短术语的解释注进词内会写出与事实相反或读不通的句子，
/// 例如把「视距」的解释注到「超视距」上（含义正好相反）、把「载波」的解释注到「副载波」上、
/// 把「信号带宽」的解释注到「必要带宽/接收带宽」上。
const BLOCKED_COMPOUNDS: [&str; 28] = [
  "超视距",
  "副载波",
  "反向击穿",
  "击穿电压",
  "线性调频",
  "音调频率",
  "话音频率",
  "调制频率",
  "解调频率",
  "调制解调",
  "上下边带",
  "抑制边带",
  "二次谐波",
  "三次谐波",
  "偶次谐波",
  "奇次谐波",
  "接收带宽",
  "通带宽度",
  "必要带宽",
  "占用带宽",
  "3dB 带宽",
  "3dB带宽",
  "高音频",
  "多载波",
  "微波射频",
  "亚音调静噪",
  "恒定包络",
  "圆极化",
];

/// 阻止注入的复合词表（字符化，便于按下标比较）。
fn blockers() -> Vec<Vec<char>> {
  BLOCKED_COMPOUNDS
    .iter()
    .map(|w| w.chars().collect())
    .collect()
}

/// 术语出现区间 `[pos, pos + len)` 是否落在某个「更长词语」内部。
fn inside_blocker(chars: &[char], pos: usize, len: usize, blockers: &[Vec<char>]) -> bool {
  let end = pos + len;
  blockers.iter().any(|b| {
    if b.len() <= len || end > chars.len() {
      return false;
    }
    (end.saturating_sub(b.len())..=pos).any(|start| {
      start + b.len() >= end && chars.get(start..start + b.len()) == Some(b.as_slice())
    })
  })
}

/// 为文本中每个术语的首次（可注入）出现追加 `（解释）`。
///
/// 规则：长术语优先；术语后已紧跟括号、位于原文括号内、与更长术语重叠、
/// 或落在 [`BLOCKED_COMPOUNDS`] 这类更长词语内部时跳过；
/// 每个术语每条解析只注入一次；「发射频率」中的「射频」不视为术语。
pub fn inject(text: &str, glossary: &[(String, String)], blockers: &[Vec<char>]) -> String {
  let chars: Vec<char> = text.chars().collect();
  let mut terms: Vec<&(String, String)> = glossary.iter().collect();
  terms.sort_by_key(|(t, _)| Reverse(t.encode_utf16().count()));

  let mut ranges = Vec::new();
  let mut stack = Vec::new();
  for (i, &ch) in chars.iter().enumerate() {
    if ch == '（' || ch == '(' {
      stack.push(i);
    } else if (ch == '）' || ch == ')')
      && let Some(open) = stack.pop()
    {
      ranges.push((open, i + 1));
    }
  }
  let in_bracket = |pos: usize| ranges.iter().any(|&(s, e)| pos >= s && pos < e);

  let mut points: Vec<(usize, usize, &str)> = Vec::new();
  let mut occupied: Vec<(usize, usize)> = Vec::new();

  for (term, explain) in terms {
    let tchars: Vec<char> = term.chars().collect();
    let len = tchars.len();
    let positions: Vec<usize> = find_all(&chars, &tchars)
      .into_iter()
      .filter(|&p| !(term == "射频" && p > 0 && chars[p - 1] == '发'))
      .collect();
    // 该术语的注入结果已在文本中则跳过：否则每次运行都会给「下一个出现位置」再补一处
    // 括注（单次运行只注入首次出现），反复 enhance 会让文本越来越长。
    // 跳过时仍要占位，否则更短的术语（如「边带」）会插进已注入的「单边带」内部。
    if text.contains(&format!("（{explain}）")) {
      occupied.extend(positions.iter().map(|&p| (p, p + len)));
      continue;
    }

    for &pos in &positions {
      let end = pos + len;
      let next = chars[end..].iter().copied().find(|&c| !is_js_whitespace(c));
      let overlaps = occupied.iter().any(|&(s, e)| pos < e && end > s);
      if !next.is_some_and(|c| OPENERS.contains(&c))
        && !in_bracket(pos)
        && !overlaps
        && !inside_blocker(&chars, pos, len, blockers)
      {
        points.push((pos, len, explain.as_str()));
        break;
      }
    }
    occupied.extend(positions.iter().map(|&p| (p, p + len)));
  }

  if points.is_empty() {
    return text.to_owned();
  }
  points.sort_by_key(|&(pos, _, _)| Reverse(pos));
  let mut out = chars;
  for (pos, len, explain) in points {
    let insert_at = pos + len;
    let add: Vec<char> = format!("（{explain}）").chars().collect();
    out.splice(insert_at..insert_at, add);
  }
  out.into_iter().collect()
}

/// 读取并校验术语表（从 `data/glossary/` 目录按分类合并）。
fn load_glossary(paths: &Paths) -> Result<Glossary> {
  let mut parts = Vec::new();
  for entry in fs::read_dir(&paths.glossary_dir)
    .with_context(|| format!("failed to read {}", paths.glossary_dir.display()))?
  {
    let path = entry?.path();
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
      parts.push(read_json(&path)?);
    }
  }
  let glossary = Glossary::merged(parts);
  let problems = glossary.validate();
  if !problems.is_empty() {
    bail!(
      "{} 校验失败：\n  - {}",
      paths.glossary_dir.display(),
      problems.join("\n  - ")
    );
  }
  Ok(glossary)
}

/// 清理历史遗留的重复术语注入：同一条解析中内容相同的括注只保留首次出现。
///
/// 注入只保证「单次运行内每个术语注入一次」，早期版本反复运行会在同一条解析里留下
/// 多处相同括注，朗读与打印都会重复。只处理 `known`（术语表释义）里的内容，
/// 长度小于 6 的括注（如「（A）」「（B）」）与人工写的括注都不参与去重。
#[must_use]
pub fn dedupe_injections(text: &str, known: &[String]) -> String {
  let chars: Vec<char> = text.chars().collect();
  let mut spans: Vec<(usize, usize, String)> = Vec::new();
  let mut start = None::<usize>;
  for (i, &ch) in chars.iter().enumerate() {
    match (ch, start) {
      // 嵌套的左括号不开新区间，按最外层处理
      ('（', None) => start = Some(i),
      ('）', Some(s)) => {
        spans.push((s, i + 1, chars[s + 1..i].iter().collect()));
        start = None;
      }
      _ => {}
    }
  }

  let mut seen: HashSet<&str> = HashSet::new();
  let mut drop: Vec<(usize, usize)> = Vec::new();
  for (s, e, content) in &spans {
    let is_injection = known.iter().any(|d| d == content);
    if is_injection && content.chars().count() >= 6 && !seen.insert(content.as_str()) {
      drop.push((*s, *e));
    }
  }
  if drop.is_empty() {
    return text.to_owned();
  }

  chars
    .iter()
    .enumerate()
    .filter(|(i, _)| !drop.iter().any(|&(s, e)| *i >= s && *i < e))
    .map(|(_, &c)| c)
    .collect()
}

/// 清理历史遗留的**错位注入**：释义被注进了更长词语内部。
///
/// 早期版本的注入只按子串匹配术语，会写出「超视距（收发两点之间看得见的直线传播）」
/// 这类与原意相反的句子，甚至把词拆开（「亚音（…）调静噪（…）」）。
/// 判定方法是先把注入的括注摘掉还原成「干净文本」，再看术语是否落在
/// [`BLOCKED_COMPOUNDS`] 这类更长词语内部——这样被括注打断的复合词也能识别。
#[must_use]
pub fn prune_injections(
  text: &str,
  glossary: &[(String, String)],
  blockers: &[Vec<char>],
) -> String {
  let chars: Vec<char> = text.chars().collect();

  // 1. 找出所有外层括注中「内容等于某词条释义」的那些（即注入产物）
  let mut stack = Vec::new();
  let mut spans: Vec<(usize, usize, String)> = Vec::new();
  for (i, &ch) in chars.iter().enumerate() {
    if ch == '（' || ch == '(' {
      stack.push(i);
    } else if (ch == '）' || ch == ')')
      && let Some(open) = stack.pop()
    {
      let content: String = chars[open + 1..i].iter().collect();
      if let Some((term, _)) = glossary.iter().find(|(_, d)| *d == content) {
        spans.push((open, i + 1, term.clone()));
      }
    }
  }
  if spans.is_empty() {
    return text.to_owned();
  }

  // 2. 还原「干净文本」，并记录原坐标 → 干净坐标的映射
  let in_span = |i: usize| spans.iter().any(|&(s, e, _)| i >= s && i < e);
  let mut map = vec![0usize; chars.len() + 1];
  let mut clean: Vec<char> = Vec::new();
  for (i, &ch) in chars.iter().enumerate() {
    map[i] = clean.len();
    if !in_span(i) {
      clean.push(ch);
    }
  }
  map[chars.len()] = clean.len();

  // 3. 干净文本里，术语若落在更长词语内部，则对应的括注要删掉
  let mut drop: Vec<(usize, usize)> = Vec::new();
  for (start, end, term) in &spans {
    let tchars: Vec<char> = term.chars().collect();
    let clean_end = map[*start];
    if clean_end < tchars.len() {
      continue;
    }
    let term_start = clean_end - tchars.len();
    if clean[term_start..clean_end] != tchars[..] {
      continue;
    }
    if inside_blocker(&clean, term_start, tchars.len(), blockers) {
      drop.push((*start, *end));
    }
  }
  if drop.is_empty() {
    return text.to_owned();
  }

  chars
    .iter()
    .enumerate()
    .filter(|(i, _)| !drop.iter().any(|&(s, e)| *i >= s && *i < e))
    .map(|(_, &c)| c)
    .collect()
}

/// 清理解析表中重复或错位的术语括注。
pub fn dedupe(paths: &Paths) -> Result<()> {
  let glossary = load_glossary(paths)?.inject_pairs();
  let known: Vec<String> = glossary.iter().map(|(_, d)| d.clone()).collect();
  let blockers = blockers();
  let mut explanations = load_ordered(&paths.explanations)?;
  let (mut changed, mut removed) = (0usize, 0usize);
  for value in explanations.values_mut() {
    let Some(before) = value.as_str() else {
      continue;
    };
    let after = dedupe_injections(&prune_injections(before, &glossary, &blockers), &known);
    if after != before {
      removed += before.matches('（').count() - after.matches('（').count();
      changed += 1;
      *value = Value::String(after);
    }
  }

  write_json(&paths.explanations, &explanations)?;
  println!("共清理 {changed} 条解析，移除重复或错位括注 {removed} 处");
  println!("当前 explanations.json 共 {} 条", explanations.len());
  println!("提示：运行 `cargo make explanations-apply` 把结果写入题库 JSON");
  Ok(())
}

/// 同一道题的多个指纹变体里，选出要保留的解析，并给出被丢弃的那些。
///
/// 规则：保留最长的一条（风格上「详细优先」）；长度相同则按指纹字典序取小，保证结果确定。
/// 只有一条或文本完全一致时不做任何改动。
#[must_use]
pub fn pick_canonical<'a>(
  texts: &[(&'a str, &'a str)],
) -> Option<(&'a str, Vec<(&'a str, &'a str)>)> {
  let first_text = texts.first()?.1;
  if texts.iter().all(|(_, text)| *text == first_text) {
    // 所有变体的文本已经完全一致：无需改动
    return None;
  }
  let (keep_fp, keep_text) = texts
    .iter()
    .copied()
    .max_by_key(|(fp, text)| (text.chars().count(), Reverse(*fp)))
    .expect("texts 非空");
  let dropped = texts
    .iter()
    .copied()
    .filter(|(fp, _)| *fp != keep_fp)
    .collect();
  Some((keep_text, dropped))
}

/// 把「同一道题的不同指纹变体」的解析文本对齐。
///
/// 题库 JSON 由 CSV 生成，同一道题在不同题库里可能因空格写法不同而得到不同指纹
/// （`ham_web_core::content_key` 忽略空白，能把它们认出来），解析表里就会留下多份
/// 内容不同、甚至互相矛盾的解析。这里按内容 key 分组，只保留最详细的一条并写回该组
/// 所有指纹；被丢弃的文本写进报告文件，便于人工确认。
pub fn sync(paths: &Paths, report: Option<&Path>) -> Result<()> {
  let banks = load_banks(paths)?;
  let mut explanations = load_ordered(&paths.explanations)?;

  // 内容 key → 指纹列表（去重，保持首次出现顺序）
  let mut order: Vec<String> = Vec::new();
  let mut groups: HashMap<String, Vec<String>> = HashMap::new();
  let mut question_of: HashMap<String, String> = HashMap::new();
  for (_, questions) in &banks {
    for q in questions {
      let key = content_key(q);
      let fp = fingerprint(q);
      let entry = groups.entry(key.clone()).or_default();
      if !entry.contains(&fp) {
        entry.push(fp);
      }
      if !order.contains(&key) {
        order.push(key.clone());
      }
      question_of.entry(key).or_insert_with(|| q.question.clone());
    }
  }

  let (mut merged, mut rewritten) = (0usize, 0usize);
  let mut report_items: Vec<Value> = Vec::new();
  for key in &order {
    let Some(fps) = groups.get(key) else {
      continue;
    };
    let texts: Vec<(String, String)> = fps
      .iter()
      .filter_map(|fp| {
        explanations
          .get(fp)
          .and_then(Value::as_str)
          .map(|text| (fp.clone(), text.to_owned()))
      })
      .collect();
    let pairs: Vec<(&str, &str)> = texts
      .iter()
      .map(|(fp, text)| (fp.as_str(), text.as_str()))
      .collect();
    let Some((keep_text, dropped)) = pick_canonical(&pairs) else {
      continue;
    };
    merged += 1;
    for (fp, _) in &texts {
      if explanations.get(fp).and_then(Value::as_str) != Some(keep_text) {
        explanations.insert(fp.clone(), Value::String(keep_text.to_owned()));
        rewritten += 1;
      }
    }
    report_items.push(serde_json::json!({
      "question": question_of.get(key),
      "kept": keep_text,
      "dropped": dropped
        .iter()
        .map(|(fp, text)| serde_json::json!({ "fingerprint": fp, "text": text }))
        .collect::<Vec<_>>(),
    }));
  }

  write_json(&paths.explanations, &explanations)?;
  println!("共发现 {merged} 组同一道题的多指纹变体，改写 {rewritten} 条解析");
  println!("当前 explanations.json 共 {} 条", explanations.len());
  if let Some(path) = report {
    write_json(path, &report_items)?;
    println!("合并明细：{}", path.display());
  }
  println!("提示：运行 `cargo make explanations-apply` 把结果写入题库 JSON");
  Ok(())
}

/// 校验术语表并输出统计。
pub fn check_glossary(paths: &Paths) -> Result<()> {
  let glossary = load_glossary(paths)?;
  let entries = glossary.entries();
  let mut by_category: Vec<(&str, usize)> = Vec::new();
  for e in entries {
    match by_category.iter_mut().find(|(c, _)| *c == e.category_key()) {
      Some((_, n)) => *n += 1,
      None => by_category.push((e.category_key(), 1)),
    }
  }
  let abbr = entries
    .iter()
    .filter(|e| e.abbreviation().is_some())
    .count();
  let inject = entries.iter().filter(|e| e.inject).count();
  println!(
    "术语表校验通过：共 {} 条，含英文缩写 {abbr} 条，参与解析注入 {inject} 条",
    entries.len()
  );
  for (c, n) in by_category {
    println!("  {c}：{n}");
  }
  Ok(())
}

/// 校验解析表：答案一致性、重复/错位注入、未加括号的释义、同题多版本分叉。
///
/// 前四类是硬错误（命令以非零状态退出，可直接挂到 CI）；同题多版本分叉只列为告警，
/// 它来自「指纹包含选项原文」的历史设计，需要人工合并而不是自动改写。
pub fn check(paths: &Paths) -> Result<()> {
  let glossary = load_glossary(paths)?.inject_pairs();
  let known: Vec<String> = glossary.iter().map(|(_, d)| d.clone()).collect();
  let blockers = blockers();
  let explanations = load_ordered(&paths.explanations)?;
  let decl = Regex::new(
    r"(?:故选|应选|应当选|答案为|答案是|正确答案是|选)\s*([A-D](?:\s*[、，,和]\s*[A-D])*)",
  )?;

  let (mut conflicts, mut misplaced, mut repeated, mut nested, mut bare) =
    (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
  for (key, value) in &explanations {
    let Some(text) = value.as_str() else {
      continue;
    };
    let Some((question, answer)) = key.rsplit_once("||") else {
      continue;
    };
    let answer: HashSet<char> = answer.chars().filter(|c| ('A'..='D').contains(c)).collect();
    let declared = declared_letters(&decl, text);
    if !declared.is_subset(&answer) {
      conflicts.push(format!(
        "{question}（答案键 {}，解析声明 {}）",
        answer.iter().collect::<String>(),
        declared.iter().collect::<String>()
      ));
    }
    if prune_injections(text, &glossary, &blockers) != text {
      misplaced.push(question.to_owned());
    }
    if dedupe_injections(text, &known) != text {
      repeated.push(question.to_owned());
    }
    if text.contains("（（") {
      nested.push(question.to_owned());
    }
    // 术语释义应当只出现在「术语（释义）」里；裸着出现说明是手抄或注入被破坏
    let chars: Vec<char> = text.chars().collect();
    let spans = paren_spans(&chars);
    for (_, desc) in &glossary {
      let needle: Vec<char> = desc.chars().collect();
      for start in find_all(&chars, &needle) {
        if start > 0 && chars[start - 1] == '（' {
          continue;
        }
        if spans.iter().any(|&(s, e)| start >= s && start < e) {
          continue;
        }
        bare.push(format!("{question}（裸释义：{desc}）"));
      }
    }
  }

  // 同一道题的多指纹变体：按内容 key（忽略空白）分组。
  // 注意不能用「题干」分组：配图题的题干可能完全相同（「下列电路是一个：」）而选项不同，
  // 那是不同的题，不是重复。
  let banks = load_banks(paths)?;
  let mut groups: HashMap<String, Vec<String>> = HashMap::new();
  for (_, questions) in &banks {
    for q in questions {
      let entry = groups.entry(content_key(q)).or_default();
      let fp = fingerprint(q);
      if !entry.contains(&fp) {
        entry.push(fp);
      }
    }
  }
  let multi_fp = groups.values().filter(|fps| fps.len() > 1).count();
  let forks: Vec<String> = groups
    .iter()
    .filter(|(_, fps)| {
      let texts: HashSet<&str> = fps
        .iter()
        .filter_map(|fp| explanations.get(fp).and_then(Value::as_str))
        .collect();
      texts.len() > 1
    })
    .map(|(key, _)| key.clone())
    .collect();

  let report = |title: &str, items: &[String]| {
    if items.is_empty() {
      println!("{title}：0");
      return;
    }
    println!("{title}：{}", items.len());
    for item in items.iter().take(5) {
      println!("  - {item}");
    }
  };
  println!("解析表校验：共 {} 条", explanations.len());
  report("答案键与解析声明矛盾", &conflicts);
  report("错位注入（释义注进更长词语内部）", &misplaced);
  report("重复注入（同一条里相同括注出现多次）", &repeated);
  report("嵌套括号（（…））", &nested);
  report("裸术语释义（未加括号）", &bare);
  report("同一道题多指纹但解析文本不一致", &forks);
  println!("提示：同指纹变体共 {multi_fp} 组，用 `cargo make explanations-sync` 对齐文本");

  if conflicts.is_empty()
    && misplaced.is_empty()
    && repeated.is_empty()
    && nested.is_empty()
    && bare.is_empty()
    && forks.is_empty()
  {
    Ok(())
  } else {
    bail!(
      "解析表存在 {} 处硬错误（答案矛盾 {}、错位注入 {}、重复注入 {}、嵌套括号 {}、裸释义 {}、多指纹分叉 {}）",
      conflicts.len() + misplaced.len() + repeated.len() + nested.len() + bare.len() + forks.len(),
      conflicts.len(),
      misplaced.len(),
      repeated.len(),
      nested.len(),
      bare.len(),
      forks.len()
    )
  }
}

/// 解析里「声明」的答案字母，例如「故选 A、C」「答案为 B」。
///
/// 「误选 D 的 0dBμV」「不要选 D」这类提醒不是声明，遇到这些前缀要跳过，
/// 否则会把解析里对干扰项的说明误判成答案矛盾。
fn declared_letters(decl: &Regex, text: &str) -> HashSet<char> {
  let mut out = HashSet::new();
  for cap in decl.captures_iter(text) {
    let start = cap.get(0).map_or(0, |m| m.start());
    if preceded_by_warning(&text[..start]) {
      continue;
    }
    out.extend(cap[1].chars().filter(|c| ('A'..='D').contains(c)));
  }
  out
}

/// 匹配起点之前的文本是否是「不要选 / 误选」这类否定或警示修饰。
fn preceded_by_warning(text_before: &str) -> bool {
  let mut rev = text_before.chars().rev();
  let last = rev.next();
  let second_last = rev.next();
  match (second_last, last) {
    // 双字否定：「不要 / 不应 / 不能 / 不可 / 不宜」选
    (Some('不'), Some('要' | '应' | '能' | '可' | '宜')) => true,
    // 单字警示：「误 / 错 / 别 / 勿 / 易 / 慎 / 不 / 非」选
    (_, Some('误' | '错' | '别' | '勿' | '易' | '慎' | '不' | '非')) => true,
    _ => false,
  }
}

/// 文本中所有括号区间 `[start, end)`（按最外层）。
fn paren_spans(chars: &[char]) -> Vec<(usize, usize)> {
  let mut stack = Vec::new();
  let mut spans = Vec::new();
  for (i, &ch) in chars.iter().enumerate() {
    if ch == '（' || ch == '(' {
      stack.push(i);
    } else if (ch == '）' || ch == ')')
      && let Some(open) = stack.pop()
    {
      spans.push((open, i + 1));
    }
  }
  spans
}

/// 用术语表中 `inject: true` 的词条增强全部解析。
pub fn enhance(paths: &Paths) -> Result<()> {
  let glossary = load_glossary(paths)?.inject_pairs();
  let blockers = blockers();
  let mut explanations = load_ordered(&paths.explanations)?;

  let (mut changed, mut total_terms) = (0usize, 0usize);
  for value in explanations.values_mut() {
    let Some(before) = value.as_str() else {
      continue;
    };
    let after = inject(before, &glossary, &blockers);
    if after != before {
      let count = |s: &str| s.matches('（').count();
      total_terms += count(&after).saturating_sub(count(before));
      changed += 1;
      *value = Value::String(after);
    }
  }

  write_json(&paths.explanations, &explanations)?;
  println!("共增强 {changed} 条解析，注入术语解释 {total_terms} 处");
  println!("当前 explanations.json 共 {} 条", explanations.len());
  Ok(())
}

/// 把解析表重新应用到 `public/questions/*.json`。
pub fn apply(paths: &Paths) -> Result<()> {
  let explanations = load_explanations(&paths.explanations);
  for name in ["A", "B", "C", "full"] {
    let file = paths.bank_json(name);
    if !file.is_file() {
      eprintln!("跳过 {}（不存在）", file.display());
      continue;
    }
    let mut qs: Vec<QuestionItem> = read_json(&file)?;
    let mut updated = 0usize;
    for q in &mut qs {
      if let Some(text) = explanations.get(&fingerprint(q)).filter(|s| !s.is_empty())
        && q.explanation.as_deref() != Some(text.as_str())
      {
        q.explanation = Some(text.clone());
        updated += 1;
      }
    }
    write_json(&file, &qs)?;
    let with = qs.iter().filter(|q| has_explanation(q)).count();
    println!(
      "{name}.json：更新 {updated} 条，含解析 {with} / {}",
      qs.len()
    );
  }
  Ok(())
}

#[derive(Serialize)]
struct MissingContext<'a> {
  id: &'a str,
  #[serde(rename = "J")]
  j: Option<&'a str>,
  #[serde(rename = "P")]
  p: Option<&'a str>,
  #[serde(rename = "type")]
  kind: &'static str,
  question: &'a str,
  options: Vec<String>,
  answer: String,
  #[serde(rename = "imageUrl", skip_serializing_if = "Option::is_none")]
  image_url: Option<&'a str>,
}

/// 统计并导出缺失解析的题目（按内容指纹去重）。
pub fn missing(
  paths: &Paths,
  only: Option<Bank>,
  output: Option<&Path>,
  context: Option<&Path>,
  limit: Option<usize>,
) -> Result<()> {
  let banks = load_banks(paths)?;
  let mut seen = HashSet::new();
  let mut template = Map::new();
  let mut contexts = Vec::new();
  let limit = limit.unwrap_or(usize::MAX);

  for (name, qs) in &banks {
    // `--bank` 只能选 A/B/C；不限题库时额外巡检 full。
    if only.is_some_and(|b| b.as_str() != name.as_str()) {
      continue;
    }
    let missing: Vec<&QuestionItem> = qs.iter().filter(|q| !has_explanation(q)).collect();
    let label = if name == "full" {
      "full 题库".to_owned()
    } else {
      format!("{name} 类")
    };
    println!(
      "{label}：共 {} 题，已有解析 {}，缺失 {}",
      qs.len(),
      qs.len() - missing.len(),
      missing.len()
    );
    for q in missing {
      // 按内容 key 去重：同一道题的空格变体只导出一条，作者不必重复填写
      if template.len() >= limit || !seen.insert(content_key(q)) {
        continue;
      }
      let Some(id) = q.id_str() else { continue };
      template.insert(id.to_owned(), Value::String(String::new()));
      contexts.push(MissingContext {
        id,
        j: q.j_code(),
        p: q.p_code(),
        kind: q.kind.label(),
        question: &q.question,
        options: q
          .options
          .iter()
          .map(|o| format!("{}. {}", o.key, o.text))
          .collect(),
        answer: q.answer_keys.concat(),
        image_url: q.image(),
      });
    }
  }
  println!("去重后待补充解析：{} 题", template.len());

  if let Some(out) = output {
    write_json(out, &template)?;
    println!(
      "已导出模板：{}（填写后执行 `cargo make explanations-add BATCH={}`）",
      out.display(),
      out.display()
    );
  }
  if let Some(out) = context {
    write_json(out, &contexts)?;
    println!("已导出题目上下文：{}", out.display());
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn g(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
      .iter()
      .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
      .collect()
  }

  #[test]
  fn injects_first_occurrence_once() {
    let glossary = g(&[("驻波比", "匹配指标")]);
    assert_eq!(
      inject("驻波比越小越好，驻波比=1", &glossary, &blockers()),
      "驻波比（匹配指标）越小越好，驻波比=1"
    );
  }

  #[test]
  fn is_idempotent_and_skips_brackets() {
    let glossary = g(&[("驻波比", "匹配指标")]);
    let once = inject("驻波比很重要", &glossary, &blockers());
    assert_eq!(inject(&once, &glossary, &blockers()), once);
    assert_eq!(inject("（驻波比）", &glossary, &blockers()), "（驻波比）");
  }

  #[test]
  fn already_injected_term_blocks_overlapping_shorter_term() {
    let glossary = g(&[("单边带", "只发一个边带"), ("边带", "载波两边多出来的信号")]);
    let once = inject("单边带省带宽，单边带常用于 DX", &glossary, &blockers());
    assert_eq!(once, "单边带（只发一个边带）省带宽，单边带常用于 DX");
    assert_eq!(inject(&once, &glossary, &blockers()), once);
  }

  #[test]
  fn skips_terms_inside_longer_compounds() {
    let glossary = g(&[
      ("视距", "收发两点之间看得见的直线传播"),
      ("载波", "承载信息的高频电波"),
      ("调频", "频率随声音变化的调制方式"),
      ("带宽", "信号占用的频率范围的宽度"),
    ]);
    let b = blockers();
    // 复合词内部不注入
    assert_eq!(
      inject("超视距传播、副载波与线性调频，必要带宽不限", &glossary, &b),
      "超视距传播、副载波与线性调频，必要带宽不限"
    );
    // 独立出现时照常注入
    assert_eq!(
      inject("视距传播靠载波，调频话音带宽有限", &glossary, &b),
      "视距（收发两点之间看得见的直线传播）传播靠载波（承载信息的高频电波），调频（频率随声音变化的调制方式）话音带宽（信号占用的频率范围的宽度）有限"
    );
  }

  #[test]
  fn prunes_misplaced_injections_only() {
    let glossary = g(&[("视距", "收发两点之间看得见的直线传播")]);
    let b = blockers();
    assert_eq!(
      prune_injections("超视距（收发两点之间看得见的直线传播）传播", &glossary, &b),
      "超视距传播"
    );
    assert_eq!(
      prune_injections("视距（收发两点之间看得见的直线传播）传播", &glossary, &b),
      "视距（收发两点之间看得见的直线传播）传播"
    );
  }

  #[test]
  fn prunes_injections_that_split_a_compound() {
    let glossary = g(&[
      ("亚音", "混在话音里的一小声低频音"),
      ("静噪", "没有信号时自动关掉喇叭"),
    ]);
    let b = blockers();
    assert_eq!(
      prune_injections(
        "指亚音（混在话音里的一小声低频音）调静噪（没有信号时自动关掉喇叭），按 67Hz 选通",
        &glossary,
        &b
      ),
      "指亚音调静噪，按 67Hz 选通"
    );
  }

  #[test]
  fn pick_canonical_keeps_the_most_detailed() {
    // 文本已经完全一致：无需改动
    assert!(pick_canonical(&[("f1", "同样一条解析"), ("f2", "同样一条解析")]).is_none());
    // 只有一个变体：无需改动
    assert!(pick_canonical(&[("f1", "唯一一条解析")]).is_none());
    // 长度不同：保留最长的一条，并列出被丢弃的
    let (keep, dropped) = pick_canonical(&[("f1", "短"), ("f2", "更长的一条解析")]).unwrap();
    assert_eq!(keep, "更长的一条解析");
    assert_eq!(dropped, vec![("f1", "短")]);
    // 长度相同：按指纹字典序取小，结果确定
    let (keep, _) = pick_canonical(&[("f2", "甲乙"), ("f1", "丙丁")]).unwrap();
    assert_eq!(keep, "丙丁");
  }

  #[test]
  fn declared_answers_skip_warning_phrases() {
    let re = Regex::new(
      r"(?:故选|应选|应当选|答案为|答案是|正确答案是|选)\s*([A-D](?:\s*[、，,和]\s*[A-D])*)",
    )
    .unwrap();
    assert_eq!(
      declared_letters(&re, "故选 A、C，其余都不对"),
      HashSet::from(['A', 'C'])
    );
    assert_eq!(
      declared_letters(&re, "把 1μV 直接当成结论会误选 D 的 0dBμV"),
      HashSet::new()
    );
    assert_eq!(
      declared_letters(&re, "本题不要选 D，0dBμV 是干扰项"),
      HashSet::new()
    );
    assert_eq!(
      declared_letters(&re, "不应选 C，这里 C 是易错项"),
      HashSet::new()
    );
    assert!(declared_letters(&re, "答案为 B").contains(&'B'));
  }

  #[test]
  fn keeps_first_injection_only() {
    let known = vec!["只发一个边带".to_owned()];
    assert_eq!(
      dedupe_injections(
        "单边带（只发一个边带）省带宽，单边带（只发一个边带）常用于 DX（A）。",
        &known
      ),
      "单边带（只发一个边带）省带宽，单边带常用于 DX（A）。"
    );
    // 短括注（答案标注）不参与去重
    assert_eq!(
      dedupe_injections("分别为（A）、（B）", &known),
      "分别为（A）、（B）"
    );
    // 不在术语表里的重复括注不动（人工写的括注）
    assert_eq!(
      dedupe_injections("示例如（如上图）与（如上图）", &known),
      "示例如（如上图）与（如上图）"
    );
  }

  #[test]
  fn converges_across_repeated_runs() {
    let glossary = g(&[("单边带", "只发一个边带"), ("驻波比", "匹配指标")]);
    let once = inject(
      "单边带省带宽，单边带常用于 DX；驻波比要低",
      &glossary,
      &blockers(),
    );
    assert_eq!(
      once,
      "单边带（只发一个边带）省带宽，单边带常用于 DX；驻波比（匹配指标）要低"
    );
    assert_eq!(inject(&once, &glossary, &blockers()), once);
    assert_eq!(
      inject(
        &inject(&once, &glossary, &blockers()),
        &glossary,
        &blockers()
      ),
      once
    );
  }

  #[test]
  fn prefers_longer_terms_and_skips_transmit_frequency() {
    let glossary = g(&[("散射", "短"), ("对流层散射", "长"), ("射频", "RF")]);
    assert_eq!(
      inject("对流层散射与发射频率", &glossary, &blockers()),
      "对流层散射（长）与发射频率"
    );
  }
}
