//! 工具与器材页的数据：器材实测值（`gear-measurements`）、DIY 项目制作进度（`diy-progress`）、
//! 噪声本底记录（`noise-floor-samples`）。

use std::collections::{BTreeMap, BTreeSet};

use ham_web_core::diy_projects::{DiyProgress, DiyProgressBook};
use ham_web_core::gear_user::{UserMeasurement, UserMeasurements};
use ham_web_core::noise_floor::Sample;
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table};

/// `gear-measurements`：机型 id → 实测值。
pub(super) struct GearMeasurementsTable(UserMeasurements);

impl Table for GearMeasurementsTable {
  const KEY: Key = Key::Exact("gear-measurements");
  const TABLES: &'static [&'static str] = &["gear_measurements"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<UserMeasurements>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO gear_measurements
         (user_id, gear_id, current_a, noise_floor_dbm, rmdr_db, phase_noise_10k_dbc, measured_on)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (gear_id, m) in self.0.iter() {
      let UserMeasurement {
        current_a,
        noise_floor_dbm,
        rmdr_db,
        phase_noise_10k_dbc,
        measured_on,
      } = m;
      stmt.execute(params![
        user,
        gear_id,
        current_a,
        noise_floor_dbm,
        rmdr_db,
        phase_noise_10k_dbc,
        measured_on
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let rows: Vec<(String, UserMeasurement)> = conn
      .prepare_cached(
        "SELECT gear_id, current_a, noise_floor_dbm, rmdr_db, phase_noise_10k_dbc, measured_on
         FROM gear_measurements WHERE user_id = ?1",
      )?
      .query_map([user], |r| {
        Ok((
          r.get(0)?,
          UserMeasurement {
            current_a: r.get(1)?,
            noise_floor_dbm: r.get(2)?,
            rmdr_db: r.get(3)?,
            phase_noise_10k_dbc: r.get(4)?,
            measured_on: r.get(5)?,
          },
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(UserMeasurements::from_entries(rows)))
  }
}

/// `diy-progress`：项目名 → 是否完成 + 已采购的材料。
pub(super) struct DiyProgressTable(DiyProgressBook);

impl Table for DiyProgressTable {
  const KEY: Key = Key::Exact("diy-progress");
  const TABLES: &'static [&'static str] = &["diy_bought", "diy_progress"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<DiyProgressBook>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut head =
      tx.prepare_cached("INSERT INTO diy_progress (user_id, project, done) VALUES (?1, ?2, ?3)")?;
    let mut item =
      tx.prepare_cached("INSERT INTO diy_bought (user_id, project, item) VALUES (?1, ?2, ?3)")?;
    for (project, progress) in self.0.iter() {
      let DiyProgress { done, bought } = progress;
      head.execute(params![user, project, done])?;
      for name in bought {
        item.execute(params![user, project, name])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut projects: BTreeMap<String, DiyProgress> = conn
      .prepare_cached("SELECT project, done FROM diy_progress WHERE user_id = ?1")?
      .query_map([user], |r| {
        Ok((
          r.get::<_, String>(0)?,
          DiyProgress {
            done: r.get(1)?,
            bought: BTreeSet::new(),
          },
        ))
      })?
      .collect::<rusqlite::Result<_>>()?;
    let items: Vec<(String, String)> = conn
      .prepare_cached("SELECT project, item FROM diy_bought WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    for (project, item) in items {
      if let Some(p) = projects.get_mut(&project) {
        p.bought.insert(item);
      }
    }
    Ok(Self(DiyProgressBook::from_entries(projects)))
  }
}

/// `noise-floor-samples`：噪声本底记录，按记录顺序。
pub(super) struct NoiseSamples(Vec<Sample>);

impl Table for NoiseSamples {
  const KEY: Key = Key::Exact("noise-floor-samples");
  const TABLES: &'static [&'static str] = &["noise_samples"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Vec<Sample>>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO noise_samples (user_id, pos, label, at_ms, floor_db, occupied_ratio)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (pos, s) in self.0.iter().enumerate() {
      let Sample {
        label,
        at_ms,
        floor_db,
        occupied_ratio,
      } = s;
      stmt.execute(params![
        user,
        pos as i64,
        label,
        at_ms,
        f64::from(*floor_db),
        f64::from(*occupied_ratio)
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .prepare_cached(
        "SELECT label, at_ms, floor_db, occupied_ratio FROM noise_samples
         WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        Ok(Sample {
          label: r.get(0)?,
          at_ms: r.get(1)?,
          floor_db: r.get::<_, f64>(2)? as f32,
          occupied_ratio: r.get::<_, f64>(3)? as f32,
        })
      })?
      .collect::<rusqlite::Result<_>>()
      .map(Self)
  }
}
