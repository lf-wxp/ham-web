//! 设置类：备考计划（`learning-plan`）、DX 热点提醒（`dx-alerts`）、卫星过境提醒（`sat-watch`）。
//!
//! 每个账号一行设置；其中的列表（关注的呼号、收藏的卫星）是子表，按 `pos` 保序。

use ham_web_core::bank::Bank;
use ham_web_core::dx_watch::AlertSettings;
use ham_web_core::sat_watch::{FavSat, SatWatch};
use ham_web_core::study_plan::StudyPlan;
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, enum_from_text, enum_text, missing};

/// `learning-plan`。
pub(super) struct StudyPlanTable(StudyPlan);

impl Table for StudyPlanTable {
  const KEY: Key = Key::Exact("learning-plan");
  const TABLES: &'static [&'static str] = &["study_plan"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<StudyPlan>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let StudyPlan {
      target_date,
      bank,
      reminder_time,
    } = &self.0;
    let bank = bank.as_ref().map(enum_text).transpose()?;
    tx.execute(
      "INSERT INTO study_plan (user_id, target_date, bank, reminder_time) VALUES (?1, ?2, ?3, ?4)",
      params![user, target_date, bank, reminder_time],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (target_date, bank, reminder_time): (String, Option<String>, String) = conn
      .query_row(
        "SELECT target_date, bank, reminder_time FROM study_plan WHERE user_id = ?1",
        [user],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
      )
      .map_err(missing("learning-plan"))?;
    Ok(Self(StudyPlan {
      target_date,
      bank: bank.as_deref().map(enum_from_text::<Bank>).transpose()?,
      reminder_time,
    }))
  }
}

/// `dx-alerts`。
pub(super) struct DxAlerts(AlertSettings);

impl Table for DxAlerts {
  const KEY: Key = Key::Exact("dx-alerts");
  const TABLES: &'static [&'static str] = &["dx_alert_calls", "dx_alerts"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<AlertSettings>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let AlertSettings {
      new_dxcc,
      new_band,
      calls,
    } = &self.0;
    tx.execute(
      "INSERT INTO dx_alerts (user_id, new_dxcc, new_band) VALUES (?1, ?2, ?3)",
      params![user, new_dxcc, new_band],
    )?;
    let mut stmt =
      tx.prepare_cached("INSERT INTO dx_alert_calls (user_id, pos, pattern) VALUES (?1, ?2, ?3)")?;
    for (pos, pattern) in calls.iter().enumerate() {
      stmt.execute(params![user, pos as i64, pattern])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (new_dxcc, new_band): (bool, bool) = conn
      .query_row(
        "SELECT new_dxcc, new_band FROM dx_alerts WHERE user_id = ?1",
        [user],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .map_err(missing("dx-alerts"))?;
    let calls = conn
      .prepare_cached("SELECT pattern FROM dx_alert_calls WHERE user_id = ?1 ORDER BY pos")?
      .query_map([user], |r| r.get::<_, String>(0))?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(AlertSettings {
      new_dxcc,
      new_band,
      calls,
    }))
  }
}

/// `sat-watch`。
pub(super) struct SatWatchTable(SatWatch);

impl Table for SatWatchTable {
  const KEY: Key = Key::Exact("sat-watch");
  const TABLES: &'static [&'static str] = &["sat_favorites", "sat_watch"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<SatWatch>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let SatWatch {
      lat,
      lon,
      min_elev,
      favorites,
      alerts,
      lead_min,
      apt_alert,
    } = &self.0;
    tx.execute(
      "INSERT INTO sat_watch (user_id, lat, lon, min_elev, alerts, lead_min, apt_alert)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![user, lat, lon, min_elev, alerts, lead_min, apt_alert],
    )?;
    let mut stmt = tx.prepare_cached(
      "INSERT INTO sat_favorites (user_id, pos, norad, name) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (pos, fav) in favorites.iter().enumerate() {
      let FavSat { norad, name } = fav;
      stmt.execute(params![user, pos as i64, norad.cast_signed(), name])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut watch = conn
      .query_row(
        "SELECT lat, lon, min_elev, alerts, lead_min, apt_alert FROM sat_watch WHERE user_id = ?1",
        [user],
        |r| {
          Ok(SatWatch {
            lat: r.get(0)?,
            lon: r.get(1)?,
            min_elev: r.get(2)?,
            favorites: Vec::new(),
            alerts: r.get(3)?,
            lead_min: r.get(4)?,
            apt_alert: r.get(5)?,
          })
        },
      )
      .map_err(missing("sat-watch"))?;
    watch.favorites = conn
      .prepare_cached("SELECT norad, name FROM sat_favorites WHERE user_id = ?1 ORDER BY pos")?
      .query_map([user], |r| {
        Ok(FavSat {
          norad: r.get::<_, i64>(0)?.cast_unsigned(),
          name: r.get(1)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(watch))
  }
}
