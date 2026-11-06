//! 页面自己的小文档：倒计时（`countdowns`）、每日打卡（`daily-checkin`）、竞赛场次（`contest-session`）、
//! 网格地图筛选（`grid-map-filters`）、听题设置（`listen-settings`）。
//!
//! 这些类型原来只定义在页面里，现在在 core（`ham_web_core::{countdown, checkin, contest, grid_filter, listen}`）。
//! 解构一律不带 `..`：给类型加了字段而忘了建列，会直接编译失败，而不是悄悄丢数据。

use std::collections::{BTreeMap, HashSet};

use ham_web_core::checkin::CheckinState;
use ham_web_core::contest::ContestSession;
use ham_web_core::countdown::{CountdownEvent, CountdownList};
use ham_web_core::grid_filter::{GridFilters, QslFilter};
use ham_web_core::listen::{Settings as ListenSettings, Source};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, db_usize, enum_from_text, missing, rust_usize};

/// `countdowns`：倒计时事件。
pub(super) struct Countdowns(CountdownList);

impl Table for Countdowns {
  const KEY: Key = Key::Exact("countdowns");
  const TABLES: &'static [&'static str] = &["countdown_events"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<CountdownList>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.events.iter().all(|e| seen.insert(e.id))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO countdown_events (user_id, id, pos, title, target_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (pos, e) in self.0.events.iter().enumerate() {
      let CountdownEvent {
        id,
        title,
        target_ms,
      } = e;
      stmt.execute(params![
        user,
        id.cast_signed(),
        pos as i64,
        title,
        target_ms
      ])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let events = conn
      .prepare_cached(
        "SELECT id, title, target_ms FROM countdown_events WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        Ok(CountdownEvent {
          id: r.get::<_, i64>(0)?.cast_unsigned(),
          title: r.get(1)?,
          target_ms: r.get(2)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(CountdownList { events }))
  }
}

/// `daily-checkin`。
pub(super) struct DailyCheckin(CheckinState);

impl Table for DailyCheckin {
  const KEY: Key = Key::Exact("daily-checkin");
  const TABLES: &'static [&'static str] = &["daily_checkin"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<CheckinState>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let CheckinState { last_date, streak } = &self.0;
    tx.execute(
      "INSERT INTO daily_checkin (user_id, last_date, streak) VALUES (?1, ?2, ?3)",
      params![user, last_date, db_usize(*streak)],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT last_date, streak FROM daily_checkin WHERE user_id = ?1",
        [user],
        |r| {
          Ok(CheckinState {
            last_date: r.get(0)?,
            streak: rust_usize(r.get(1)?),
          })
        },
      )
      .map(Self)
      .map_err(missing("daily-checkin"))
  }
}

/// `contest-session`。
pub(super) struct ContestSessionTable(ContestSession);

impl Table for ContestSessionTable {
  const KEY: Key = Key::Exact("contest-session");
  const TABLES: &'static [&'static str] = &["contest_session"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<ContestSession>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let ContestSession {
      contest_id,
      start,
      end,
      station_id,
      my_exch,
      freq,
      mode,
      power,
      target_qsos,
      planned_minutes,
    } = &self.0;
    tx.execute(
      "INSERT INTO contest_session
         (user_id, contest_id, start, end_at, station_id, my_exch, freq, mode, power,
          target_qsos, planned_minutes)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
      params![
        user,
        contest_id,
        start,
        end,
        station_id.cast_signed(),
        my_exch,
        freq,
        mode,
        power,
        target_qsos,
        planned_minutes
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT contest_id, start, end_at, station_id, my_exch, freq, mode, power,
                target_qsos, planned_minutes
         FROM contest_session WHERE user_id = ?1",
        [user],
        |r| {
          Ok(ContestSession {
            contest_id: r.get(0)?,
            start: r.get(1)?,
            end: r.get(2)?,
            station_id: r.get::<_, i64>(3)?.cast_unsigned(),
            my_exch: r.get(4)?,
            freq: r.get(5)?,
            mode: r.get(6)?,
            power: r.get(7)?,
            target_qsos: r.get(8)?,
            planned_minutes: r.get(9)?,
          })
        },
      )
      .map(Self)
      .map_err(missing("contest-session"))
  }
}

/// `grid-map-filters`。
pub(super) struct GridFiltersTable(GridFilters);

impl Table for GridFiltersTable {
  const KEY: Key = Key::Exact("grid-map-filters");
  const TABLES: &'static [&'static str] = &["grid_filters"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<GridFilters>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let GridFilters {
      band,
      mode,
      dxcc,
      year,
      callsign,
      qsl,
      paths,
    } = &self.0;
    tx.execute(
      "INSERT INTO grid_filters (user_id, band, mode, dxcc, year, callsign, qsl, paths)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        user,
        band,
        mode,
        dxcc,
        year,
        callsign,
        qsl.map(QslFilter::as_str),
        paths
      ],
    )?;
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .query_row(
        "SELECT band, mode, dxcc, year, callsign, qsl, paths FROM grid_filters WHERE user_id = ?1",
        [user],
        |r| {
          Ok(GridFilters {
            band: r.get(0)?,
            mode: r.get(1)?,
            dxcc: r.get(2)?,
            year: r.get(3)?,
            callsign: r.get(4)?,
            qsl: r
              .get::<_, Option<String>>(5)?
              .as_deref()
              .map(enum_from_text::<QslFilter>)
              .transpose()?,
            paths: r.get(6)?,
          })
        },
      )
      .map(Self)
      .map_err(missing("grid-map-filters"))
  }
}

/// `listen-settings`。
pub(super) struct ListenSettingsTable(ListenSettings);

impl Table for ListenSettingsTable {
  const KEY: Key = Key::Exact("listen-settings");
  const TABLES: &'static [&'static str] = &["listen_positions", "listen_settings"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<ListenSettings>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let ListenSettings {
      source,
      rate,
      think_secs,
      explain,
      shuffle,
      positions,
    } = &self.0;
    tx.execute(
      "INSERT INTO listen_settings (user_id, source, rate, think_secs, explain, shuffle)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        user,
        source.as_str(),
        f64::from(*rate),
        think_secs,
        explain,
        shuffle
      ],
    )?;
    let mut stmt = tx.prepare_cached(
      "INSERT INTO listen_positions (user_id, source, position) VALUES (?1, ?2, ?3)",
    )?;
    for (source, position) in positions {
      stmt.execute(params![user, source.as_str(), db_usize(*position)])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let (source, rate, think_secs, explain, shuffle) = conn
      .query_row(
        "SELECT source, rate, think_secs, explain, shuffle FROM listen_settings WHERE user_id = ?1",
        [user],
        |r| {
          Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, u32>(2)?,
            r.get::<_, bool>(3)?,
            r.get::<_, bool>(4)?,
          ))
        },
      )
      .map_err(missing("listen-settings"))?;
    let rows: Vec<(String, i64)> = conn
      .prepare_cached("SELECT source, position FROM listen_positions WHERE user_id = ?1")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    let positions = rows
      .into_iter()
      .map(|(s, p)| Ok((enum_from_text::<Source>(&s)?, rust_usize(p))))
      .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    Ok(Self(ListenSettings {
      source: enum_from_text(&source)?,
      rate: rate as f32,
      think_secs,
      explain,
      shuffle,
      positions,
    }))
  }
}
