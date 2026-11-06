//! `question-stats`：每道题（按内容指纹）的累计作答次数、正确次数、最近作答时间与耗时。

use std::collections::BTreeMap;

use ham_web_core::question_stats::{QuestionStat, QuestionStats};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table};

pub(super) struct QuestionStatsTable(QuestionStats);

impl Table for QuestionStatsTable {
  const KEY: Key = Key::Exact("question-stats");
  const TABLES: &'static [&'static str] = &["question_stats"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<QuestionStats>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO question_stats (user_id, question_key, attempts, correct, last_ms, duration_ms)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (key, s) in &self.0.items {
      let QuestionStat {
        attempts,
        correct,
        last_ms,
        duration_ms,
      } = s;
      stmt.execute(params![
        user,
        key,
        attempts,
        correct,
        last_ms,
        duration_ms.cast_signed()
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let items = conn
      .prepare_cached(
        "SELECT question_key, attempts, correct, last_ms, duration_ms
         FROM question_stats WHERE user_id = ?1",
      )?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          QuestionStat {
            attempts: r.get(1)?,
            correct: r.get(2)?,
            last_ms: r.get(3)?,
            duration_ms: r.get::<_, i64>(4)?.cast_unsigned(),
          },
        ))
      })?
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(QuestionStats { items }))
  }
}
