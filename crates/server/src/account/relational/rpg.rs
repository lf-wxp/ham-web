//! 闯关玩法：关卡历史最佳星级（`rpg-stars`）与怪物图鉴（`rpg-bestiary`）。

use std::collections::{BTreeMap, BTreeSet};

use ham_web_core::rpg::{Bestiary, StageStars};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table};

/// `rpg-stars`：关卡 id → 历史最佳星级。
pub(super) struct RpgStars(StageStars);

impl Table for RpgStars {
  const KEY: Key = Key::Exact("rpg-stars");
  const TABLES: &'static [&'static str] = &["stage_stars"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<StageStars>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt =
      tx.prepare_cached("INSERT INTO stage_stars (user_id, stage, stars) VALUES (?1, ?2, ?3)")?;
    for (stage, stars) in &self.0.best {
      stmt.execute(params![user, stage, stars])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let best = conn
      .prepare_cached("SELECT stage, stars FROM stage_stars WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u8>(1)?)))?
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(StageStars { best }))
  }
}

/// `rpg-bestiary`：已击败怪物（错题 key）的集合。
pub(super) struct RpgBestiary(Bestiary);

impl Table for RpgBestiary {
  const KEY: Key = Key::Exact("rpg-bestiary");
  const TABLES: &'static [&'static str] = &["bestiary"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Bestiary>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt =
      tx.prepare_cached("INSERT INTO bestiary (user_id, question_key) VALUES (?1, ?2)")?;
    for key in &self.0.defeated {
      stmt.execute(params![user, key])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let defeated = conn
      .prepare_cached("SELECT question_key FROM bestiary WHERE user_id = ?1")?
      .query_map([user], |r| r.get::<_, String>(0))?
      .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok(Self(Bestiary { defeated }))
  }
}
