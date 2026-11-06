//! 练习与考试的断点进度：`practice:{版本}:{题库}`、`exam:savedState:{版本}:{题库}`。
//!
//! 每个 key 是表里自己的一行（认领一族 key，见 [`Key::Prefix`]）。`key` 列存完整的存储 key，
//! 入表前核对它与进度里的题库 / 版本一致——不一致（或根本不是进度，如 `practice:lastMode`）的
//! 原样留在目录里。按题位的作答、题序、标记是长度等于题数的数组，整体读写，存成 JSON 文本列。

use ham_web_core::bank::Bank;
use ham_web_core::saved_state::{ExamSavedState, PracticeSavedState};
use rusqlite::{Connection, Transaction, params};

use super::{
  Key, Table, db_usize, enum_from_text, enum_text, from_json, missing, rust_usize, to_json,
};

/// 练习进度。
pub(super) struct PracticeProgress(PracticeSavedState);

impl Table for PracticeProgress {
  const KEY: Key = Key::Prefix("practice:");
  const TABLES: &'static [&'static str] = &["practice_progress"];

  fn decode(key: &str, raw: &str) -> Option<Self> {
    let state = serde_json::from_str::<PracticeSavedState>(raw).ok()?;
    (PracticeSavedState::storage_key(state.bank, state.version_id.as_deref()) == key)
      .then_some(Self(state))
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    let PracticeSavedState {
      version,
      bank,
      version_id,
      timestamp,
      index,
      order,
      show_answer,
      show_explanation,
      order_indices,
      answers_by_position,
      total,
    } = &self.0;
    tx.execute(
      "INSERT INTO practice_progress
         (user_id, key, version, bank, version_id, timestamp, current_index, order_mode,
          show_answer, show_explanation, total, order_indices, answers)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
      params![
        user,
        key,
        version,
        enum_text(bank)?,
        version_id,
        timestamp,
        db_usize(*index),
        order,
        show_answer,
        show_explanation,
        db_usize(*total),
        to_json(order_indices)?,
        to_json(answers_by_position)?
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT version, bank, version_id, timestamp, current_index, order_mode, show_answer,
                show_explanation, total, order_indices, answers
         FROM practice_progress WHERE user_id = ?1 AND key = ?2",
        params![user, key],
        |r| {
          Ok(PracticeSavedState {
            version: r.get(0)?,
            bank: enum_from_text::<Bank>(&r.get::<_, String>(1)?)?,
            version_id: r.get(2)?,
            timestamp: r.get(3)?,
            index: rust_usize(r.get(4)?),
            order: r.get(5)?,
            show_answer: r.get(6)?,
            show_explanation: r.get(7)?,
            total: rust_usize(r.get(8)?),
            order_indices: from_json(&r.get::<_, String>(9)?, 9)?,
            answers_by_position: from_json(&r.get::<_, String>(10)?, 10)?,
          })
        },
      )
      .map(Self)
      .map_err(missing(&format!("练习进度 {key}")))
  }

  fn clear(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<()> {
    conn
      .execute(
        "DELETE FROM practice_progress WHERE user_id = ?1 AND key = ?2",
        params![user, key],
      )
      .map(|_| ())
  }
}

/// 考试进度。
pub(super) struct ExamProgress(ExamSavedState);

impl Table for ExamProgress {
  const KEY: Key = Key::Prefix("exam:savedState:");
  const TABLES: &'static [&'static str] = &["exam_progress"];

  fn decode(key: &str, raw: &str) -> Option<Self> {
    let state = serde_json::from_str::<ExamSavedState>(raw).ok()?;
    (ExamSavedState::storage_key(state.bank, state.version_id.as_deref()) == key)
      .then_some(Self(state))
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    let ExamSavedState {
      version,
      bank,
      version_id,
      timestamp,
      end_at_ms,
      index,
      answers_by_position,
      flags_by_position,
      total,
      question_ids,
      questions_snapshot,
    } = &self.0;
    // 两个可选数组：没有（NULL）与「有但是空」（`[]`）是两回事，原文里分得开，库里也要分得开。
    let question_ids = question_ids.as_ref().map(to_json).transpose()?;
    let questions_snapshot = questions_snapshot.as_ref().map(to_json).transpose()?;
    tx.execute(
      "INSERT INTO exam_progress
         (user_id, key, version, bank, version_id, timestamp, end_at_ms, current_index,
          answers, flags, total, question_ids, questions_snapshot)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
      params![
        user,
        key,
        version,
        enum_text(bank)?,
        version_id,
        timestamp,
        end_at_ms,
        db_usize(*index),
        to_json(answers_by_position)?,
        to_json(flags_by_position)?,
        db_usize(*total),
        question_ids,
        questions_snapshot
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT version, bank, version_id, timestamp, end_at_ms, current_index, answers, flags,
                total, question_ids, questions_snapshot
         FROM exam_progress WHERE user_id = ?1 AND key = ?2",
        params![user, key],
        |r| {
          let json = |col: usize| r.get::<_, Option<String>>(col);
          Ok(ExamSavedState {
            version: r.get(0)?,
            bank: enum_from_text::<Bank>(&r.get::<_, String>(1)?)?,
            version_id: r.get(2)?,
            timestamp: r.get(3)?,
            end_at_ms: r.get(4)?,
            index: rust_usize(r.get(5)?),
            answers_by_position: from_json(&r.get::<_, String>(6)?, 6)?,
            flags_by_position: from_json(&r.get::<_, String>(7)?, 7)?,
            total: rust_usize(r.get(8)?),
            question_ids: json(9)?.map(|t| from_json(&t, 9)).transpose()?,
            questions_snapshot: json(10)?.map(|t| from_json(&t, 10)).transpose()?,
          })
        },
      )
      .map(Self)
      .map_err(missing(&format!("考试进度 {key}")))
  }

  fn clear(conn: &Connection, user: i64, key: &str) -> rusqlite::Result<()> {
    conn
      .execute(
        "DELETE FROM exam_progress WHERE user_id = ?1 AND key = ?2",
        params![user, key],
      )
      .map(|_| ())
  }
}
