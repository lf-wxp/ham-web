//! 摩尔斯页面的四份小文档：解码练习统计（`morse-stats`）、解码练习设置（`morse-trainer`）、
//! 发报统计（`morse-send`）、播放设置（`morse-settings`）。类型在 `ham_web_core::morse_store`。

use std::collections::BTreeMap;

use ham_web_core::morse_store::{MorseSettings, MorseStats, SendStats, TrainerSettings};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, db_usize, enum_from_text, enum_text, missing, rust_usize};

/// `morse-stats`：总数 + 易错字符计数 + 每日快照。
pub(super) struct MorseStatsTable(MorseStats);

impl Table for MorseStatsTable {
  const KEY: Key = Key::Exact("morse-stats");
  const TABLES: &'static [&'static str] = &["morse_daily", "morse_mistakes", "morse_stats"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<MorseStats>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let MorseStats {
      correct,
      wrong,
      mistakes,
      // 当前连续答对只在运行时存在（`#[serde(skip)]`），不入库。
      streak: _,
      best_streak,
      top_wpm,
      today,
      today_correct,
      today_wrong,
      daily,
    } = &self.0;
    tx.execute(
      "INSERT INTO morse_stats
         (user_id, correct, wrong, best_streak, top_wpm, today, today_correct, today_wrong)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        user,
        db_usize(*correct),
        db_usize(*wrong),
        db_usize(*best_streak),
        top_wpm,
        today,
        db_usize(*today_correct),
        db_usize(*today_wrong)
      ],
    )?;
    let mut stmt =
      tx.prepare_cached("INSERT INTO morse_mistakes (user_id, ch, count) VALUES (?1, ?2, ?3)")?;
    for (ch, n) in mistakes {
      stmt.execute(params![user, ch, db_usize(*n)])?;
    }
    let mut stmt = tx.prepare_cached(
      "INSERT INTO morse_daily (user_id, day, correct, wrong) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (day, (c, w)) in daily {
      stmt.execute(params![user, day, db_usize(*c), db_usize(*w)])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut stats = conn
      .query_row(
        "SELECT correct, wrong, best_streak, top_wpm, today, today_correct, today_wrong
         FROM morse_stats WHERE user_id = ?1",
        [user],
        |r| {
          Ok(MorseStats {
            correct: rust_usize(r.get(0)?),
            wrong: rust_usize(r.get(1)?),
            mistakes: BTreeMap::new(),
            streak: 0,
            best_streak: rust_usize(r.get(2)?),
            top_wpm: r.get(3)?,
            today: r.get(4)?,
            today_correct: rust_usize(r.get(5)?),
            today_wrong: rust_usize(r.get(6)?),
            daily: BTreeMap::new(),
          })
        },
      )
      .map_err(missing("morse-stats"))?;
    stats.mistakes = conn
      .prepare_cached("SELECT ch, count FROM morse_mistakes WHERE user_id = ?1")?
      .query_map([user], |r| {
        Ok((r.get::<_, String>(0)?, rust_usize(r.get(1)?)))
      })?
      .collect::<rusqlite::Result<_>>()?;
    stats.daily = conn
      .prepare_cached("SELECT day, correct, wrong FROM morse_daily WHERE user_id = ?1")?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          (rust_usize(r.get(1)?), rust_usize(r.get(2)?)),
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(stats))
  }
}

/// `morse-trainer`。
pub(super) struct TrainerSettingsTable(TrainerSettings);

impl Table for TrainerSettingsTable {
  const KEY: Key = Key::Exact("morse-trainer");
  const TABLES: &'static [&'static str] = &["morse_trainer"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<TrainerSettings>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let TrainerSettings {
      mode,
      scope,
      target,
      wpm,
      eff_wpm,
      adaptive,
    } = &self.0;
    tx.execute(
      "INSERT INTO morse_trainer (user_id, mode, scope, target, wpm, eff_wpm, adaptive)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        user,
        enum_text(mode)?,
        enum_text(scope)?,
        enum_text(target)?,
        wpm,
        eff_wpm,
        adaptive
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT mode, scope, target, wpm, eff_wpm, adaptive FROM morse_trainer WHERE user_id = ?1",
        [user],
        |r| {
          Ok(TrainerSettings {
            mode: enum_from_text(&r.get::<_, String>(0)?)?,
            scope: enum_from_text(&r.get::<_, String>(1)?)?,
            target: enum_from_text(&r.get::<_, String>(2)?)?,
            wpm: r.get(3)?,
            eff_wpm: r.get(4)?,
            adaptive: r.get(5)?,
          })
        },
      )
      .map(Self)
      .map_err(missing("morse-trainer"))
  }
}

/// `morse-send`。
pub(super) struct SendStatsTable(SendStats);

impl Table for SendStatsTable {
  const KEY: Key = Key::Exact("morse-send");
  const TABLES: &'static [&'static str] = &["morse_send"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<SendStats>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let SendStats {
      correct,
      wrong,
      best_streak,
    } = &self.0;
    tx.execute(
      "INSERT INTO morse_send (user_id, correct, wrong, best_streak) VALUES (?1, ?2, ?3, ?4)",
      params![
        user,
        db_usize(*correct),
        db_usize(*wrong),
        db_usize(*best_streak)
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT correct, wrong, best_streak FROM morse_send WHERE user_id = ?1",
        [user],
        |r| {
          Ok(SendStats {
            correct: rust_usize(r.get(0)?),
            wrong: rust_usize(r.get(1)?),
            best_streak: rust_usize(r.get(2)?),
          })
        },
      )
      .map(Self)
      .map_err(missing("morse-send"))
  }
}

/// `morse-settings`。
pub(super) struct MorseSettingsTable(MorseSettings);

impl Table for MorseSettingsTable {
  const KEY: Key = Key::Exact("morse-settings");
  const TABLES: &'static [&'static str] = &["morse_settings"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<MorseSettings>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let MorseSettings { tone_hz, volume } = &self.0;
    tx.execute(
      "INSERT INTO morse_settings (user_id, tone_hz, volume) VALUES (?1, ?2, ?3)",
      params![user, tone_hz, f64::from(*volume)],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT tone_hz, volume FROM morse_settings WHERE user_id = ?1",
        [user],
        |r| {
          Ok(MorseSettings {
            tone_hz: r.get(0)?,
            volume: r.get::<_, f64>(1)? as f32,
          })
        },
      )
      .map(Self)
      .map_err(missing("morse-settings"))
  }
}
