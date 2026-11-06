//! 莫尔斯训练：Koch 法抄收进度（`morse-koch`）与 CW 呼号抄收成绩（`morse-runner`）。
//!
//! 两份数据都是「一行标量 + 一张按字符计数的子表」；字符是 JSON 对象的键，库里存成长度为 1 的文本。

use std::collections::BTreeMap;

use ham_web_core::cw_runner::RunnerStats;
use ham_web_core::koch::{CharTally, KochProgress};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, db_usize, parse_char, rust_usize};

/// `morse-koch`。
pub(super) struct KochProgressTable(KochProgress);

impl Table for KochProgressTable {
  const KEY: Key = Key::Exact("morse-koch");
  const TABLES: &'static [&'static str] = &["koch_chars", "koch_progress"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<KochProgress>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let KochProgress {
      level,
      char_wpm,
      effective_wpm,
      chars,
      level_tally,
    } = &self.0;
    tx.execute(
      "INSERT INTO koch_progress
         (user_id, level, char_wpm, effective_wpm, level_sent, level_correct)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        user,
        db_usize(*level),
        char_wpm,
        effective_wpm,
        level_tally.sent,
        level_tally.correct
      ],
    )?;
    let mut stmt = tx.prepare_cached(
      "INSERT INTO koch_chars (user_id, ch, sent, correct) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (ch, t) in chars {
      stmt.execute(params![user, ch.to_string(), t.sent, t.correct])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (level, char_wpm, effective_wpm, level_tally) = conn
      .query_row(
        "SELECT level, char_wpm, effective_wpm, level_sent, level_correct
         FROM koch_progress WHERE user_id = ?1",
        [user],
        |r| {
          Ok((
            rust_usize(r.get(0)?),
            r.get::<_, u32>(1)?,
            r.get::<_, u32>(2)?,
            CharTally {
              sent: r.get(3)?,
              correct: r.get(4)?,
            },
          ))
        },
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => corrupt("morse-koch 标为专表存放，但没有进度行"),
        other => other,
      })?;
    let rows: Vec<(String, CharTally)> = conn
      .prepare_cached("SELECT ch, sent, correct FROM koch_chars WHERE user_id = ?1")?
      .query_map([user], |r| {
        Ok((
          r.get(0)?,
          CharTally {
            sent: r.get(1)?,
            correct: r.get(2)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    let chars = rows
      .into_iter()
      .map(|(ch, t)| Ok((parse_char(&ch)?, t)))
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(KochProgress {
      level,
      char_wpm,
      effective_wpm,
      chars,
      level_tally,
    }))
  }
}

/// `morse-runner`。
pub(super) struct RunnerStatsTable(RunnerStats);

impl Table for RunnerStatsTable {
  const KEY: Key = Key::Exact("morse-runner");
  const TABLES: &'static [&'static str] = &["runner_missed", "runner_stats"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<RunnerStats>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let RunnerStats {
      wpm,
      adaptive,
      cut_numbers,
      pileup,
      sessions,
      qsos,
      calls_ok,
      best_score,
      top_wpm,
      missed,
    } = &self.0;
    tx.execute(
      "INSERT INTO runner_stats
         (user_id, wpm, adaptive, cut_numbers, pileup, sessions, qsos, calls_ok, best_score, top_wpm)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
      params![
        user, wpm, adaptive, cut_numbers, pileup, sessions, qsos, calls_ok, best_score, top_wpm
      ],
    )?;
    let mut stmt =
      tx.prepare_cached("INSERT INTO runner_missed (user_id, ch, missed) VALUES (?1, ?2, ?3)")?;
    for (ch, n) in missed {
      stmt.execute(params![user, ch.to_string(), n])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut stats = conn
      .query_row(
        "SELECT wpm, adaptive, cut_numbers, pileup, sessions, qsos, calls_ok, best_score, top_wpm
         FROM runner_stats WHERE user_id = ?1",
        [user],
        |r| {
          Ok(RunnerStats {
            wpm: r.get(0)?,
            adaptive: r.get(1)?,
            cut_numbers: r.get(2)?,
            pileup: r.get(3)?,
            sessions: r.get(4)?,
            qsos: r.get(5)?,
            calls_ok: r.get(6)?,
            best_score: r.get(7)?,
            top_wpm: r.get(8)?,
            missed: BTreeMap::new(),
          })
        },
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => corrupt("morse-runner 标为专表存放，但没有成绩行"),
        other => other,
      })?;
    let rows: Vec<(String, u32)> = conn
      .prepare_cached("SELECT ch, missed FROM runner_missed WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (ch, n) in rows {
      stats.missed.insert(parse_char(&ch)?, n);
    }
    Ok(Self(stats))
  }
}
