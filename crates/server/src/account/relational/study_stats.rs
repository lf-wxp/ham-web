//! `study-stats`：按分类累计的作答数与正确数，外加各题库「做过的题」。
//!
//! 范围（scope）= 全部（`bank` 为空串）或某个题库（`A` / `B` / `C`）；每个范围有自己的总数、
//! 一级分类与官方分类码的分项，题库范围还有做过的题集合。分项与做过的题挂在范围上，
//! 删范围时一并级联删除。

use ham_web_core::bank::Bank;
use ham_web_core::mistake_book::{BankStats, CategoryStats, StudyStats, Tally};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, enum_from_text, enum_text};

pub(super) struct StudyStatsTable(StudyStats);

fn insert_scope(
  tx: &Transaction<'_>,
  user: i64,
  bank: &str,
  stats: &CategoryStats,
) -> rusqlite::Result<()> {
  let CategoryStats {
    categories,
    subs,
    answered,
    correct,
  } = stats;
  tx.prepare_cached(
    "INSERT INTO study_scopes (user_id, bank, answered, correct) VALUES (?1, ?2, ?3, ?4)",
  )?
  .execute(params![user, bank, answered, correct])?;
  let mut stmt = tx.prepare_cached(
    "INSERT INTO study_tallies (user_id, bank, kind, key, answered, correct)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
  )?;
  for (kind, map) in [("category", categories), ("sub", subs)] {
    for (key, t) in map {
      stmt.execute(params![user, bank, kind, key, t.answered, t.correct])?;
    }
  }
  Ok(())
}

fn scope_mut<'a>(stats: &'a mut StudyStats, bank: &str) -> rusqlite::Result<&'a mut BankStats> {
  let bank: Bank = enum_from_text(bank)?;
  stats
    .banks
    .get_mut(&bank)
    .ok_or_else(|| corrupt("study-stats 的分项指向不存在的题库范围"))
}

impl Table for StudyStatsTable {
  const KEY: Key = Key::Exact("study-stats");
  const TABLES: &'static [&'static str] = &["study_seen", "study_tallies", "study_scopes"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<StudyStats>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let StudyStats { total, banks } = &self.0;
    insert_scope(tx, user, "", total)?;
    for (bank, bs) in banks {
      let BankStats { stats, seen } = bs;
      let bank = enum_text(bank)?;
      insert_scope(tx, user, &bank, stats)?;
      let mut stmt = tx.prepare_cached(
        "INSERT INTO study_seen (user_id, bank, question_key) VALUES (?1, ?2, ?3)",
      )?;
      for key in seen {
        stmt.execute(params![user, bank, key])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut out = StudyStats::default();
    let scopes: Vec<(String, u32, u32)> = conn
      .prepare_cached("SELECT bank, answered, correct FROM study_scopes WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (bank, answered, correct) in scopes {
      let target = if bank.is_empty() {
        &mut out.total
      } else {
        let bank: Bank = enum_from_text(&bank)?;
        &mut out.banks.entry(bank).or_default().stats
      };
      target.answered = answered;
      target.correct = correct;
    }

    let tallies: Vec<(String, String, String, Tally)> = conn
      .prepare_cached(
        "SELECT bank, kind, key, answered, correct FROM study_tallies WHERE user_id = ?1",
      )?
      .query_map([user], |r| {
        Ok((
          r.get(0)?,
          r.get(1)?,
          r.get(2)?,
          Tally {
            answered: r.get(3)?,
            correct: r.get(4)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    for (bank, kind, key, tally) in tallies {
      let target = if bank.is_empty() {
        &mut out.total
      } else {
        &mut scope_mut(&mut out, &bank)?.stats
      };
      match kind.as_str() {
        "category" => target.categories.insert(key, tally),
        "sub" => target.subs.insert(key, tally),
        other => return Err(corrupt(format!("未知的分项种类 {other}"))),
      };
    }

    let seen: Vec<(String, String)> = conn
      .prepare_cached("SELECT bank, question_key FROM study_seen WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (bank, key) in seen {
      scope_mut(&mut out, &bank)?.seen.insert(key);
    }
    Ok(Self(out))
  }
}
