//! `exam-history`：模拟考试成绩（最多 50 条，时间升序）。
//!
//! 记录没有天然的唯一键（同一秒交两份卷是可能的），所以主键只用 `pos`；
//! 顺序本身就是数据的一部分。

use ham_web_core::exam_history::ExamRecord;
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table};

pub(super) struct ExamResults(Vec<ExamRecord>);

impl Table for ExamResults {
  const KEY: Key = Key::Exact("exam-history");
  const TABLES: &'static [&'static str] = &["exam_results"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Vec<ExamRecord>>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO exam_results (user_id, pos, bank, correct, total, timestamp, weak)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (pos, r) in self.0.iter().enumerate() {
      stmt.execute(params![
        user,
        pos as i64,
        r.bank,
        (r.correct as u64).cast_signed(),
        (r.total as u64).cast_signed(),
        r.timestamp,
        r.weak
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .prepare_cached(
        "SELECT bank, correct, total, timestamp, weak FROM exam_results
         WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        Ok(ExamRecord {
          bank: r.get(0)?,
          correct: usize::try_from(r.get::<_, i64>(1)?.cast_unsigned()).unwrap_or(usize::MAX),
          total: usize::try_from(r.get::<_, i64>(2)?.cast_unsigned()).unwrap_or(usize::MAX),
          timestamp: r.get(3)?,
          weak: r.get(4)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()
      .map(Self)
  }
}
