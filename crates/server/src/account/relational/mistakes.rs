//! `mistake-book`：错题本（最多 1000 条）。
//!
//! 一行一道错题，`key`（题目内容指纹）账号内唯一才入表。题目快照拆成列 + 子表：
//! 题面与元数据是 `mistakes` 的列，选项、标准答案、我的作答、错过的题库各是一张子表。
//! `Option` 字段用 `NULL` 表示「没有」，与空串分得开。

use std::collections::{BTreeSet, HashMap, HashSet};

use ham_web_core::bank::Bank;
use ham_web_core::mistake_book::{MistakeBook, MistakeRecord};
use ham_web_core::question::{Codes, QuestionItem, QuestionOption, QuestionType};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, enum_from_text, enum_text, pages_cols, pages_of};

pub(super) struct Mistakes(MistakeBook);

impl Table for Mistakes {
  const KEY: Key = Key::Exact("mistake-book");
  const TABLES: &'static [&'static str] = &[
    "mistake_banks",
    "mistake_choices",
    "mistake_options",
    "mistakes",
  ];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<MistakeBook>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.records.iter().all(|r| seen.insert(r.key.as_str()))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut head = tx.prepare_cached(
      "INSERT INTO mistakes
         (user_id, key, pos, q_id, code_j, code_p, question, kind, image_url, explanation,
          has_pages, pages_start, pages_end, wrong_count, streak, last_wrong_ms, due_ms,
          ease, interval_days, last_review_ms, cause)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
               ?18, ?19, ?20, ?21)",
    )?;
    let mut option = tx.prepare_cached(
      "INSERT INTO mistake_options (user_id, key, pos, opt_key, text) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut choice = tx.prepare_cached(
      "INSERT INTO mistake_choices (user_id, key, kind, pos, value) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut bank =
      tx.prepare_cached("INSERT INTO mistake_banks (user_id, key, bank) VALUES (?1, ?2, ?3)")?;

    for (pos, record) in self.0.records.iter().enumerate() {
      // 解构不带 `..`：给这两个类型加了字段而忘了建列，会直接编译失败而不是悄悄丢数据。
      let MistakeRecord {
        key,
        question,
        my_answer,
        wrong_count,
        streak,
        last_wrong_ms,
        due_ms,
        banks,
        ease,
        interval_days,
        last_review_ms,
        cause,
      } = record;
      let QuestionItem {
        id,
        codes,
        question: text,
        options,
        answer_keys,
        kind,
        pages,
        image_url,
        explanation,
      } = question;
      let (has_pages, pages_start, pages_end) = pages_cols(*pages);
      head.execute(params![
        user,
        key,
        pos as i64,
        id,
        codes.j,
        codes.p,
        text,
        enum_text(kind)?,
        image_url,
        explanation,
        has_pages,
        pages_start,
        pages_end,
        wrong_count,
        streak,
        last_wrong_ms,
        due_ms,
        ease,
        interval_days,
        last_review_ms,
        cause
      ])?;
      for (i, o) in options.iter().enumerate() {
        option.execute(params![user, key, i as i64, o.key, o.text])?;
      }
      for (kind, values) in [("answer", answer_keys), ("mine", my_answer)] {
        for (i, v) in values.iter().enumerate() {
          choice.execute(params![user, key, kind, i as i64, v])?;
        }
      }
      for b in banks {
        bank.execute(params![user, key, enum_text(b)?])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut records: Vec<MistakeRecord> = conn
      .prepare_cached(
        "SELECT key, q_id, code_j, code_p, question, kind, image_url, explanation,
                has_pages, pages_start, pages_end, wrong_count, streak, last_wrong_ms, due_ms,
                ease, interval_days, last_review_ms, cause
         FROM mistakes WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        let pages = pages_of(r.get(8)?, r.get(9)?, r.get(10)?);
        Ok(MistakeRecord {
          key: r.get(0)?,
          question: QuestionItem {
            id: r.get(1)?,
            codes: Codes {
              j: r.get(2)?,
              p: r.get(3)?,
            },
            question: r.get(4)?,
            options: Vec::new(),
            answer_keys: Vec::new(),
            kind: enum_from_text::<QuestionType>(&r.get::<_, String>(5)?)?,
            pages,
            image_url: r.get(6)?,
            explanation: r.get(7)?,
          },
          my_answer: Vec::new(),
          wrong_count: r.get(11)?,
          streak: r.get(12)?,
          last_wrong_ms: r.get(13)?,
          due_ms: r.get(14)?,
          banks: BTreeSet::new(),
          ease: r.get(15)?,
          interval_days: r.get(16)?,
          last_review_ms: r.get(17)?,
          cause: r.get(18)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;

    // 子表各读一遍，按 key 挂回去；比一条记录查三次少了几千次往返。
    let at: HashMap<String, usize> = records
      .iter()
      .enumerate()
      .map(|(i, r)| (r.key.clone(), i))
      .collect();
    let slot = |key: &str| at.get(key).copied();

    let options: Vec<(String, QuestionOption)> = conn
      .prepare_cached(
        "SELECT key, opt_key, text FROM mistake_options WHERE user_id = ?1 ORDER BY key, pos",
      )?
      .query_map([user], |r| {
        Ok((
          r.get(0)?,
          QuestionOption {
            key: r.get(1)?,
            text: r.get(2)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    for (key, option) in options {
      if let Some(i) = slot(&key) {
        records[i].question.options.push(option);
      }
    }

    let choices: Vec<(String, String, String)> = conn
      .prepare_cached(
        "SELECT key, kind, value FROM mistake_choices WHERE user_id = ?1 ORDER BY key, kind, pos",
      )?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (key, kind, value) in choices {
      if let Some(i) = slot(&key) {
        match kind.as_str() {
          "answer" => records[i].question.answer_keys.push(value),
          _ => records[i].my_answer.push(value),
        }
      }
    }

    let banks: Vec<(String, String)> = conn
      .prepare_cached("SELECT key, bank FROM mistake_banks WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (key, bank) in banks {
      if let Some(i) = slot(&key) {
        records[i].banks.insert(enum_from_text::<Bank>(&bank)?);
      }
    }
    Ok(Self(MistakeBook { records }))
  }
}
