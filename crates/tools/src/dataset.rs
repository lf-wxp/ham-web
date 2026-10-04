//! 数据集构建：CSV → `public/questions/{A,B,C,full}.json` + `public/questions/images/*.jpg`。
//!
//! 数据源优先读取本地目录（`--dataset-dir` / `DATASET_DIR`），缺失时从远程仓库下载。
//! 解析文本来自 `data/explanations.json`（以内容指纹为 key），CSV 中的解析列作为兜底。
//! 题面文本在建库时统一排版（见 `ham_web_core::typography`），保证同一道题在各题库里
//! 逐字节一致，不会因空格写法不同而分叉出多个指纹。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ham_web_core::fingerprint::fingerprint_parts;
use ham_web_core::typography::normalize;
use ham_web_core::{
  Codes, QuestionItem, QuestionOption, QuestionType, fingerprint, normalize_question,
};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::csv::Table;
use crate::explanations::{self, load_explanations};
use crate::fsutil::{Paths, read_json, write_json};

const MAX_BODY: u64 = 64 * 1024 * 1024;

/// 数据源：本地目录优先，其次远程。
pub struct Source<'a> {
  local: Option<&'a Path>,
  remote: &'a str,
  images: HashMap<String, Vec<u8>>,
}

impl<'a> Source<'a> {
  pub fn new(local: Option<&'a Path>, remote: &'a str) -> Self {
    Self {
      local,
      remote: remote.trim_end_matches('/'),
      images: HashMap::new(),
    }
  }

  fn local_path(&self, rel: &str) -> Option<PathBuf> {
    self.local.map(|d| d.join(rel)).filter(|p| p.is_file())
  }

  fn url(&self, rel: &str) -> String {
    format!("{}/{rel}", self.remote)
  }

  pub fn text(&self, rel: &str) -> Result<String> {
    if let Some(p) = self.local_path(rel) {
      return fs::read_to_string(&p).with_context(|| format!("failed to read {}", p.display()));
    }
    let url = self.url(rel);
    let mut res = ureq::get(&url)
      .call()
      .with_context(|| format!("Fetch failed: {url}"))?;
    res
      .body_mut()
      .with_config()
      .limit(MAX_BODY)
      .read_to_string()
      .with_context(|| format!("Fetch failed: {url}"))
  }

  pub fn binary(&mut self, rel: &str) -> Result<Vec<u8>> {
    if let Some(hit) = self.images.get(rel) {
      return Ok(hit.clone());
    }
    let data = if let Some(p) = self.local_path(rel) {
      fs::read(&p).with_context(|| format!("failed to read {}", p.display()))?
    } else {
      let url = self.url(rel);
      let mut res = ureq::get(&url)
        .call()
        .with_context(|| format!("Fetch failed: {url}"))?;
      res
        .body_mut()
        .with_config()
        .limit(MAX_BODY)
        .read_to_vec()
        .with_context(|| format!("Fetch failed: {url}"))?
    };
    self.images.insert(rel.to_owned(), data.clone());
    Ok(data)
  }
}

/// 去除选项末尾的图片脚注，如 `[F]LK0500.jpg`、`[F]LK0942.jpg179`。
fn sanitizer() -> Regex {
  Regex::new(r"(?i)\s*\[F\][^\s,]+\.jpg\d*").expect("valid regex")
}

const BANKS: [(&str, &str); 4] = [
  ("A", "class_a.csv"),
  ("B", "class_b.csv"),
  ("C", "class_c.csv"),
  ("full", "full.csv"),
];

struct BuildCtx<'a> {
  source: Source<'a>,
  images_by_j: HashMap<String, String>,
  explanations: HashMap<String, String>,
  sanitize: Regex,
  img_dir: PathBuf,
}

impl BuildCtx<'_> {
  fn option(&self, table_row: &[String], col: Option<usize>) -> String {
    let raw = Table::get(table_row, col);
    normalize(self.sanitize.replace_all(raw, "").trim())
  }

  fn process_bank(&mut self, bank_key: &str, csv_name: &str) -> Result<Vec<QuestionItem>> {
    let table = Table::parse(&self.source.text(csv_name)?);
    let col = |n: &str| table.col(n);
    let (cj, cp, cq, ct) = (col("J"), col("P"), col("Q"), col("T"));
    let opt_cols = [
      ("A", col("A")),
      ("B", col("B")),
      ("C", col("C")),
      ("D", col("D")),
    ];
    let explanation_col = ["Explanation", "explanation", "解析", "analysis", "Analysis"]
      .iter()
      .find_map(|n| table.col(n));

    let mut out: Vec<QuestionItem> = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
      let jraw = Table::get(row, cj);
      let primary_j = jraw.split(',').next().unwrap_or_default().trim();
      let p = Table::get(row, cp);
      let q = normalize(Table::get(row, cq));
      let t = Table::get(row, ct);
      if q.is_empty() {
        continue;
      }
      let options: Vec<QuestionOption> = opt_cols
        .iter()
        .filter_map(|&(key, c)| {
          let text = self.option(row, c);
          (!text.is_empty()).then(|| QuestionOption {
            key: key.to_owned(),
            text,
          })
        })
        .collect();
      let answer_keys: Vec<String> = t
        .chars()
        .filter(char::is_ascii_uppercase)
        .map(String::from)
        .collect();
      let kind = if answer_keys.len() <= 1 {
        QuestionType::Single
      } else {
        QuestionType::Multiple
      };
      let id = format!("{bank_key}-{}", out.len() + 1);

      let explanation = self
        .explanations
        .get(&fingerprint_parts(&q, &options, &answer_keys))
        .filter(|s| !s.is_empty())
        .cloned()
        .or_else(|| {
          let v = Table::get(row, explanation_col);
          (!v.is_empty()).then(|| normalize(v))
        });

      let mut image_url = None;
      if !primary_j.is_empty()
        && let Some(image_rel) = self.images_by_j.get(primary_j).cloned()
      {
        let base = Path::new(&image_rel)
          .file_name()
          .and_then(|s| s.to_str())
          .unwrap_or(&image_rel)
          .to_owned();
        // images.csv 中的 ImagePath（images/N.jpg）只是占位符，真实图片位于 images_2/{J}.jpg。
        let data = self.source.binary(&format!("images_2/{primary_j}.jpg"))?;
        fs::write(self.img_dir.join(&base), data)?;
        image_url = Some(format!("/questions/images/{base}"));
      }

      out.push(QuestionItem {
        id: Some(id),
        codes: Codes {
          j: (!jraw.is_empty()).then(|| jraw.to_owned()),
          p: (!p.is_empty()).then(|| p.to_owned()),
        },
        question: q.to_owned(),
        options,
        answer_keys,
        kind,
        pages: None,
        image_url,
        explanation,
      });
    }
    Ok(out)
  }
}

/// 刷新 `public/questions/` 下由题库文本派生的文件（搜索索引）。
///
/// `config.json` 里的版本列表是手工维护的，`rev` 只在构建后处理写入 `dist/`，这里不动它。
fn refresh_derived_artifacts(paths: &Paths) -> Result<()> {
  crate::postbuild::write_question_search_index(&paths.public)
}

/// 构建数据集。
pub fn build(paths: &Paths, local: Option<&Path>, remote: &str) -> Result<()> {
  println!("Building dataset for static hosting...");
  if let Some(dir) = local
    && !dir.is_dir()
  {
    bail!("DATASET_DIR {} 不存在", dir.display());
  }
  let img_dir = paths.questions.join("images");
  fs::create_dir_all(&img_dir)?;

  let source = Source::new(local, remote);
  let images = Table::parse(&source.text("images.csv")?);
  let (cj, cpath) = (images.col("J"), images.col("ImagePath"));
  let images_by_j = images
    .rows
    .iter()
    .filter_map(|r| {
      let (j, p) = (Table::get(r, cj), Table::get(r, cpath));
      (!j.is_empty() && !p.is_empty()).then(|| (j.to_owned(), p.to_owned()))
    })
    .collect();

  let mut ctx = BuildCtx {
    source,
    images_by_j,
    explanations: load_explanations(&paths.explanations),
    sanitize: sanitizer(),
    img_dir,
  };

  for (bank_key, csv_name) in BANKS {
    let out = ctx.process_bank(bank_key, csv_name)?;
    write_json(&paths.bank_json(bank_key), &out)?;
    let with_expl = out.iter().filter(|q| q.explanation.is_some()).count();
    println!(
      "Wrote /questions/{bank_key}.json ({}, 含解析 {with_expl})",
      out.len()
    );
  }
  refresh_derived_artifacts(paths)?;
  println!("Dataset build complete.");
  Ok(())
}

/// 一次性迁移：把已提交的 `public/questions/*.json` 按排版规则归一化，并同步迁移
/// `data/explanations.json` 的 key。
///
/// `cargo make dataset` 建库时已经会归一化，这个命令用于**不重新下载 CSV** 也能把现有
/// 数据迁移过来（与 `explanations-apply` 同类）。同一道题因空格写法不同而产生的多个
/// 指纹会合并成一条解析（保留最长的一条），术语表释义与解析正文也一并归一化。
pub fn normalize_committed(paths: &Paths, report: Option<&Path>) -> Result<()> {
  let mut legacy_to_new: HashMap<String, String> = HashMap::new();
  let mut banks: Vec<(String, Vec<QuestionItem>)> = Vec::new();
  let mut changed_questions = 0usize;

  for name in ["A", "B", "C", "full"] {
    let file = paths.bank_json(name);
    let mut questions: Vec<QuestionItem> = read_json(&file).with_context(|| {
      format!(
        "题库 {} 不存在或损坏，请先运行 `cargo make dataset`",
        file.display()
      )
    })?;
    for q in &mut questions {
      let legacy = fingerprint(q);
      if normalize_question(q) {
        changed_questions += 1;
      }
      let current = fingerprint(q);
      if legacy != current {
        legacy_to_new.insert(legacy, current);
      }
    }
    banks.push((name.to_owned(), questions));
  }

  // 解析表：key 迁移 + 正文归一化 + 变体合并
  let table: Map<String, Value> = if paths.explanations.is_file() {
    read_json(&paths.explanations)?
  } else {
    Map::new()
  };
  let (mut rekeyed, mut merged) = (0usize, 0usize);
  let mut merges: Vec<Value> = Vec::new();
  let mut migrated: Map<String, Value> = Map::new();
  for (old_key, value) in &table {
    let text = normalize(value.as_str().unwrap_or_default());
    let new_key = legacy_to_new
      .get(old_key)
      .cloned()
      .unwrap_or_else(|| old_key.clone());
    if new_key != *old_key {
      rekeyed += 1;
    }
    match migrated.get_mut(&new_key) {
      None => {
        migrated.insert(new_key, Value::String(text));
      }
      Some(existing) => {
        let prev = existing.as_str().unwrap_or_default().to_owned();
        let (keep, dropped) = if prev.chars().count() >= text.chars().count() {
          (prev, text)
        } else {
          (text, prev)
        };
        merges.push(json!({ "fingerprint": new_key, "kept": keep, "dropped": dropped }));
        *existing = Value::String(keep);
        merged += 1;
      }
    }
  }

  // 术语表释义：保持与解析正文同一套排版（否则注入/清理匹配不上）
  let mut glossary_files = 0usize;
  for entry in fs::read_dir(&paths.glossary_dir)
    .with_context(|| format!("读取 {} 失败", paths.glossary_dir.display()))?
  {
    let path = entry?.path();
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
      continue;
    }
    let mut map: Map<String, Value> = read_json(&path)?;
    let mut changed = false;
    for value in map.values_mut() {
      let normalized = match value.get("desc").and_then(Value::as_str) {
        Some(desc) => Some(("desc", normalize(desc), desc.to_owned())),
        None => value.as_str().map(|s| ("", normalize(s), s.to_owned())),
      };
      if let Some((field, text, before)) = normalized
        && text != before
      {
        if field.is_empty() {
          *value = Value::String(text);
        } else {
          value["desc"] = Value::String(text);
        }
        changed = true;
      }
    }
    if changed {
      write_json(&path, &map)?;
      glossary_files += 1;
    }
  }

  write_json(&paths.explanations, &migrated)?;
  for (name, questions) in &banks {
    write_json(&paths.bank_json(name), questions)?;
  }
  if let Some(path) = report {
    write_json(
      path,
      &json!({
        "changedQuestions": changed_questions,
        "rekeyed": rekeyed,
        "merged": merged,
        "glossaryFiles": glossary_files,
        "merges": merges,
      }),
    )?;
  }

  println!(
    "排版归一化：改写 {changed_questions} 道题，迁移解析 key {rekeyed} 条，合并变体 {merged} 组"
  );
  if glossary_files > 0 {
    println!("术语表释义同步：{glossary_files} 个文件");
  }
  println!("解析表当前 {} 条", migrated.len());
  if let Some(path) = report {
    println!("迁移明细：{}", path.display());
  }
  explanations::apply(paths)?;
  refresh_derived_artifacts(paths)?;
  println!("提示：再运行 `cargo make explanations-check` 复核");
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_image_footnotes() {
    let re = sanitizer();
    assert_eq!(re.replace_all("选项 [F]LK0500.jpg", "").trim(), "选项");
    assert_eq!(re.replace_all("x [f]LK0942.jpg179", "").trim(), "x");
  }
}
