//! 按日期记的两份记录：每日作答量（`study-daily`）与每日挑战成绩（`daily-challenge`）。
//!
//! 日期（`YYYY-MM-DD`）就是天然主键：同一天只有一行。

use std::collections::BTreeMap;

use ham_web_core::daily_challenge::{DailyResult, DailyResults};
use ham_web_core::study_plan::{DailyLog, DayTally};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, db_usize, rust_usize};

/// `study-daily`：每天的作答量（保留 60 天）。
pub(super) struct StudyDaily(DailyLog);

impl Table for StudyDaily {
  const KEY: Key = Key::Exact("study-daily");
  const TABLES: &'static [&'static str] = &["study_daily"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<DailyLog>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO study_daily (user_id, day, answered, new_count, duration_ms)
       VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (day, t) in &self.0.days {
      let DayTally {
        answered,
        new,
        duration_ms,
      } = t;
      stmt.execute(params![user, day, answered, new, duration_ms.cast_signed()])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let days = conn
      .prepare_cached(
        "SELECT day, answered, new_count, duration_ms FROM study_daily WHERE user_id = ?1",
      )?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          DayTally {
            answered: r.get(1)?,
            new: r.get(2)?,
            duration_ms: r.get::<_, i64>(3)?.cast_unsigned(),
          },
        ))
      })?
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(DailyLog { days }))
  }
}

/// `daily-challenge`：每日挑战成绩（保留 90 天）。
pub(super) struct DailyChallenge(DailyResults);

impl Table for DailyChallenge {
  const KEY: Key = Key::Exact("daily-challenge");
  const TABLES: &'static [&'static str] = &["daily_challenge"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<DailyResults>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO daily_challenge (user_id, day, correct, total) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (day, r) in &self.0.days {
      let DailyResult { correct, total } = r;
      stmt.execute(params![user, day, db_usize(*correct), db_usize(*total)])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let days = conn
      .prepare_cached("SELECT day, correct, total FROM daily_challenge WHERE user_id = ?1")?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          DailyResult {
            correct: rust_usize(r.get(1)?),
            total: rust_usize(r.get(2)?),
          },
        ))
      })?
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(DailyResults { days }))
  }
}
