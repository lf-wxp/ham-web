//! `station-book`：台站档案册（一次安装可有多个台站）。
//!
//! 档案册头（下一个 id、当前台站）与档案各一张表。`id` 是本机自增的标识，
//! 同一账号内唯一才入表；重复（手改或合并出的坏数据）整个 key 退回原样存放。

use std::collections::HashSet;

use ham_web_core::logbook::StationInfo;
use ham_web_core::station::{StationBook, StationProfile};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, missing};

pub(super) struct Stations(StationBook);

impl Table for Stations {
  const KEY: Key = Key::Exact("station-book");
  const TABLES: &'static [&'static str] = &["station_profiles", "station_books"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<StationBook>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.profiles.iter().all(|p| seen.insert(p.id))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    tx.execute(
      "INSERT INTO station_books (user_id, next_id, active_id) VALUES (?1, ?2, ?3)",
      params![
        user,
        self.0.next_id.cast_signed(),
        self.0.active_id.cast_signed()
      ],
    )?;
    let mut stmt = tx.prepare_cached(
      "INSERT INTO station_profiles
         (user_id, id, pos, label, callsign, operator, gridsquare, rig, antenna)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for (pos, p) in self.0.profiles.iter().enumerate() {
      stmt.execute(params![
        user,
        p.id.cast_signed(),
        pos as i64,
        p.label,
        p.callsign,
        p.operator,
        p.gridsquare,
        p.rig,
        p.antenna
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (next_id, active_id): (i64, i64) = conn
      .query_row(
        "SELECT next_id, active_id FROM station_books WHERE user_id = ?1",
        [user],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
          corrupt("station-book 标为专表存放，但没有档案册头")
        }
        other => other,
      })?;
    let profiles = conn
      .prepare_cached(
        "SELECT id, label, callsign, operator, gridsquare, rig, antenna
         FROM station_profiles WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        Ok(StationProfile {
          id: r.get::<_, i64>(0)?.cast_unsigned(),
          label: r.get(1)?,
          callsign: r.get(2)?,
          operator: r.get(3)?,
          gridsquare: r.get(4)?,
          rig: r.get(5)?,
          antenna: r.get(6)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(StationBook {
      next_id: next_id.cast_unsigned(),
      profiles,
      active_id: active_id.cast_unsigned(),
    }))
  }
}

/// `station-info`：本台信息，是当前台站的镜像（老的单值写法，RBN / PSK Reporter 等页面还在读它）。
pub(super) struct StationInfoTable(StationInfo);

impl Table for StationInfoTable {
  const KEY: Key = Key::Exact("station-info");
  const TABLES: &'static [&'static str] = &["station_info"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<StationInfo>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let StationInfo {
      callsign,
      operator,
      gridsquare,
      rig,
      antenna,
    } = &self.0;
    tx.execute(
      "INSERT INTO station_info (user_id, callsign, operator, gridsquare, rig, antenna)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![user, callsign, operator, gridsquare, rig, antenna],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT callsign, operator, gridsquare, rig, antenna FROM station_info WHERE user_id = ?1",
        [user],
        |r| {
          Ok(StationInfo {
            callsign: r.get(0)?,
            operator: r.get(1)?,
            gridsquare: r.get(2)?,
            rig: r.get(3)?,
            antenna: r.get(4)?,
          })
        },
      )
      .map(Self)
      .map_err(missing("station-info"))
  }
}
