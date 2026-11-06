//! `card-review`：知识卡片的复习进度。
//!
//! 一份进度 = 一个头（今天是哪天）+ 每张卡一行 + 今天各卡组已引入的新卡数。
//! 卡片 id 形如 `q:QRM`（前缀是卡组），本身就是自然键。

use std::collections::BTreeMap;

use ham_web_core::card_review::{CardState, Deck, Schedule};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, enum_from_text, enum_text};

pub(super) struct CardReviews(Schedule);

impl Table for CardReviews {
  const KEY: Key = Key::Exact("card-review");
  const TABLES: &'static [&'static str] = &["card_new_today", "card_states", "card_schedule"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Schedule>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let Schedule {
      cards,
      new_day,
      new_today,
    } = &self.0;
    tx.execute(
      "INSERT INTO card_schedule (user_id, new_day) VALUES (?1, ?2)",
      params![user, new_day],
    )?;
    let mut stmt = tx.prepare_cached(
      "INSERT INTO card_states (user_id, id, ease, interval_days, due_ms, last_ms, reps, lapses)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for (id, c) in cards {
      let CardState {
        ease,
        interval_days,
        due_ms,
        last_ms,
        reps,
        lapses,
      } = c;
      stmt.execute(params![
        user,
        id,
        ease,
        interval_days,
        due_ms,
        last_ms,
        reps,
        lapses
      ])?;
    }
    let mut stmt =
      tx.prepare_cached("INSERT INTO card_new_today (user_id, deck, count) VALUES (?1, ?2, ?3)")?;
    for (deck, count) in new_today {
      stmt.execute(params![user, enum_text(deck)?, count])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let new_day: String = conn
      .query_row(
        "SELECT new_day FROM card_schedule WHERE user_id = ?1",
        [user],
        |r| r.get(0),
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => corrupt("card-review 标为专表存放，但没有进度头"),
        other => other,
      })?;
    let cards = conn
      .prepare_cached(
        "SELECT id, ease, interval_days, due_ms, last_ms, reps, lapses
         FROM card_states WHERE user_id = ?1",
      )?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          CardState {
            ease: r.get(1)?,
            interval_days: r.get(2)?,
            due_ms: r.get(3)?,
            last_ms: r.get(4)?,
            reps: r.get(5)?,
            lapses: r.get(6)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    let decks: Vec<(String, u32)> = conn
      .prepare_cached("SELECT deck, count FROM card_new_today WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    let new_today = decks
      .into_iter()
      .map(|(deck, n)| Ok((enum_from_text::<Deck>(&deck)?, n)))
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(Schedule {
      cards,
      new_day,
      new_today,
    }))
  }
}
