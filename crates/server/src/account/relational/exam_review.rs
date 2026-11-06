//! `exam:last-review`：最近一次模拟考试的逐题复盘（只保留一次，至多 90 题）。
//!
//! 一个头（题库、交卷时间）+ 每题一行；每题的选项、标准答案、用户作答是各自的子表。

use std::collections::BTreeMap;

use ham_web_core::exam_review::{ExamReview, ReviewItem};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt};

pub(super) struct ExamReviews(ExamReview);

impl Table for ExamReviews {
  const KEY: Key = Key::Exact("exam:last-review");
  const TABLES: &'static [&'static str] = &[
    "exam_review_choices",
    "exam_review_options",
    "exam_review_items",
    "exam_reviews",
  ];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<ExamReview>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let ExamReview {
      bank,
      finished_at_ms,
      items,
    } = &self.0;
    tx.execute(
      "INSERT INTO exam_reviews (user_id, bank, finished_at_ms) VALUES (?1, ?2, ?3)",
      params![user, bank, finished_at_ms],
    )?;
    let mut item_stmt = tx.prepare_cached(
      "INSERT INTO exam_review_items (user_id, pos, id, code, question, explanation, category)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut option_stmt = tx.prepare_cached(
      "INSERT INTO exam_review_options (user_id, pos, opt_pos, text) VALUES (?1, ?2, ?3, ?4)",
    )?;
    let mut choice_stmt = tx.prepare_cached(
      "INSERT INTO exam_review_choices (user_id, pos, kind, choice_pos, value)
       VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (pos, item) in items.iter().enumerate() {
      let pos = pos as i64;
      let ReviewItem {
        id,
        code,
        question,
        options,
        answer,
        given,
        explanation,
        category,
      } = item;
      item_stmt.execute(params![
        user,
        pos,
        id,
        code,
        question,
        explanation,
        category
      ])?;
      for (i, text) in options.iter().enumerate() {
        option_stmt.execute(params![user, pos, i as i64, text])?;
      }
      for (kind, values) in [("answer", answer), ("given", given)] {
        for (i, v) in values.iter().enumerate() {
          choice_stmt.execute(params![user, pos, kind, i as i64, v])?;
        }
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (bank, finished_at_ms) = conn
      .query_row(
        "SELECT bank, finished_at_ms FROM exam_reviews WHERE user_id = ?1",
        [user],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
          corrupt("exam:last-review 标为专表存放，但没有复盘头")
        }
        other => other,
      })?;

    // pos → 该题的各个列表；子表按 (pos, 序号) 读出来，顺序就是原顺序。
    let mut options: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for row in conn
      .prepare_cached(
        "SELECT pos, text FROM exam_review_options WHERE user_id = ?1 ORDER BY pos, opt_pos",
      )?
      .query_map([user], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
      })?
    {
      let (pos, text) = row?;
      options.entry(pos).or_default().push(text);
    }
    let mut choices: BTreeMap<(i64, String), Vec<String>> = BTreeMap::new();
    for row in conn
      .prepare_cached(
        "SELECT pos, kind, value FROM exam_review_choices
         WHERE user_id = ?1 ORDER BY pos, kind, choice_pos",
      )?
      .query_map([user], |r| {
        Ok((
          r.get::<_, i64>(0)?,
          r.get::<_, String>(1)?,
          r.get::<_, String>(2)?,
        ))
      })?
    {
      let (pos, kind, value) = row?;
      choices.entry((pos, kind)).or_default().push(value);
    }

    let items = conn
      .prepare_cached(
        "SELECT pos, id, code, question, explanation, category
         FROM exam_review_items WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        let pos: i64 = r.get(0)?;
        Ok(ReviewItem {
          id: r.get(1)?,
          code: r.get(2)?,
          question: r.get(3)?,
          options: options.remove(&pos).unwrap_or_default(),
          answer: choices
            .remove(&(pos, "answer".to_owned()))
            .unwrap_or_default(),
          given: choices
            .remove(&(pos, "given".to_owned()))
            .unwrap_or_default(),
          explanation: r.get(4)?,
          category: r.get(5)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;
    // 挂不上题目的子行：库被改坏。静默丢成空数组会让复盘少掉选项或答案。
    if let Some(pos) = options
      .keys()
      .chain(choices.keys().map(|(pos, _)| pos))
      .next()
    {
      return Err(corrupt(format!("复盘第 {pos} 项的子行没有对应的题目")));
    }
    Ok(Self(ExamReview {
      bank,
      finished_at_ms,
      items,
    }))
  }
}
