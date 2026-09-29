//! 题目解析维护：合并、术语增强、缺失统计、重新应用。
//!
//! 解析统一存放在 `data/explanations.json`，key 为题目内容指纹（见 `ham_web_core::fingerprint`），
//! value 为解析文本。指纹与题号/顺序/题库无关，因此题库重排不会导致解析错位，
//! A/B/C 中内容相同的题目共享同一条解析。

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use ham_web_core::glossary::Glossary;
use ham_web_core::text::{is_js_whitespace, js_trim};
use ham_web_core::{Bank, QuestionItem, fingerprint};
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

fn load_banks(paths: &Paths) -> Result<Vec<(Bank, Vec<QuestionItem>)>> {
  Bank::ALL
    .iter()
    .map(|&b| Ok((b, read_json(&paths.bank_json(b.as_str()))?)))
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
      Value::String(s) => js_trim(s).to_owned(),
      other => js_trim(&other.to_string()).to_owned(),
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

/// 为文本中每个术语的首次（可注入）出现追加 `（解释）`。
///
/// 规则：长术语优先；术语后已紧跟括号、位于原文括号内、或与更长术语重叠时跳过；
/// 每个术语每条解析只注入一次；「发射频率」中的「射频」不视为术语。
pub fn inject(text: &str, glossary: &[(String, String)]) -> String {
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

    for &pos in &positions {
      let end = pos + len;
      let next = chars[end..].iter().copied().find(|&c| !is_js_whitespace(c));
      let overlaps = occupied.iter().any(|&(s, e)| pos < e && end > s);
      if !next.is_some_and(|c| OPENERS.contains(&c)) && !in_bracket(pos) && !overlaps {
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

/// 读取并校验术语表。
fn load_glossary(paths: &Paths) -> Result<Glossary> {
  let glossary: Glossary = read_json(&paths.glossary)?;
  let problems = glossary.validate();
  if !problems.is_empty() {
    bail!(
      "{} 校验失败：\n  - {}",
      paths.glossary.display(),
      problems.join("\n  - ")
    );
  }
  Ok(glossary)
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

/// 用术语表中 `inject: true` 的词条增强全部解析。
pub fn enhance(paths: &Paths) -> Result<()> {
  let glossary = load_glossary(paths)?.inject_pairs();
  let mut explanations = load_ordered(&paths.explanations)?;

  let (mut changed, mut total_terms) = (0usize, 0usize);
  for value in explanations.values_mut() {
    let Some(before) = value.as_str() else {
      continue;
    };
    let after = inject(before, &glossary);
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

  for (bank, qs) in &banks {
    if only.is_some_and(|b| b != *bank) {
      continue;
    }
    let missing: Vec<&QuestionItem> = qs.iter().filter(|q| !has_explanation(q)).collect();
    println!(
      "{bank} 类：共 {} 题，已有解析 {}，缺失 {}",
      qs.len(),
      qs.len() - missing.len(),
      missing.len()
    );
    for q in missing {
      if template.len() >= limit || !seen.insert(fingerprint(q)) {
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
      inject("驻波比越小越好，驻波比=1", &glossary),
      "驻波比（匹配指标）越小越好，驻波比=1"
    );
  }

  #[test]
  fn is_idempotent_and_skips_brackets() {
    let glossary = g(&[("驻波比", "匹配指标")]);
    let once = inject("驻波比很重要", &glossary);
    assert_eq!(inject(&once, &glossary), once);
    assert_eq!(inject("（驻波比）", &glossary), "（驻波比）");
  }

  #[test]
  fn prefers_longer_terms_and_skips_transmit_frequency() {
    let glossary = g(&[("散射", "短"), ("对流层散射", "长"), ("射频", "RF")]);
    assert_eq!(
      inject("对流层散射与发射频率", &glossary),
      "对流层散射（长）与发射频率"
    );
  }
}
