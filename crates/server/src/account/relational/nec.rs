//! `nec-designs`：NEC 天线设计存档。
//!
//! 一份设计 = 一行 + 导线 / 馈电 / 负载 / 传输线四张子表。数组下标（`pos`）是数据的一部分：
//! 馈电、负载、传输线用导线的**下标**引用导线，所以子表都按 `pos` 保序。
//! 设计的 `id` 是存档册内自增的编号，重复（手改的坏数据）时整个 key 留在目录里原样存放。

use std::collections::{BTreeMap, HashSet};

use ham_web_core::nec_designs::{
  DesignBook, SavedDesign, SavedFeed, SavedGround, SavedLine, SavedLoad, SavedModel, SavedWire,
};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table, corrupt, db_usize, rust_usize};

pub(super) struct NecDesigns(DesignBook);

/// `nec_designs` 的一行（子表的内容另外按 `design_id` 分组读）。
struct DesignRow {
  id: i64,
  name: String,
  saved_at: String,
  freq_hz: f64,
  ground: String,
  eps_r: Option<f64>,
  sigma: Option<f64>,
  conductivity: Option<f64>,
}

fn ground_cols(g: &SavedGround) -> (&'static str, Option<f64>, Option<f64>) {
  match *g {
    SavedGround::Free => ("free", None, None),
    SavedGround::Perfect => ("perfect", None, None),
    SavedGround::Lossy { eps_r, sigma } => ("lossy", Some(eps_r), Some(sigma)),
  }
}

fn ground_of(kind: &str, eps_r: Option<f64>, sigma: Option<f64>) -> rusqlite::Result<SavedGround> {
  match (kind, eps_r, sigma) {
    ("free", _, _) => Ok(SavedGround::Free),
    ("perfect", _, _) => Ok(SavedGround::Perfect),
    ("lossy", Some(eps_r), Some(sigma)) => Ok(SavedGround::Lossy { eps_r, sigma }),
    _ => Err(corrupt(format!("大地 {kind} 的参数不全"))),
  }
}

/// 某张子表按设计分组读出：`design_id → 行（按 pos）`。
fn group<T>(
  conn: &Connection,
  user: i64,
  sql: &str,
  row: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<BTreeMap<i64, Vec<T>>> {
  let mut out: BTreeMap<i64, Vec<T>> = BTreeMap::new();
  let mut stmt = conn.prepare_cached(sql)?;
  let mut rows = stmt.query([user])?;
  while let Some(r) = rows.next()? {
    out.entry(r.get(0)?).or_default().push(row(r)?);
  }
  Ok(out)
}

impl Table for NecDesigns {
  const KEY: Key = Key::Exact("nec-designs");
  const TABLES: &'static [&'static str] = &[
    "nec_lines",
    "nec_loads",
    "nec_feeds",
    "nec_wires",
    "nec_designs",
  ];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<DesignBook>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.designs.iter().all(|d| seen.insert(d.id))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut head = tx.prepare_cached(
      "INSERT INTO nec_designs
         (user_id, id, pos, name, saved_at, freq_hz, ground, eps_r, sigma, conductivity)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    let mut wire = tx.prepare_cached(
      "INSERT INTO nec_wires
         (user_id, design_id, pos, ax, ay, az, bx, by, bz, radius, segments)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?;
    let mut feed = tx.prepare_cached(
      "INSERT INTO nec_feeds (user_id, design_id, pos, wire, at, volts)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut load = tx.prepare_cached(
      "INSERT INTO nec_loads (user_id, design_id, pos, wire, at, r_ohm, l_uh, c_pf)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    let mut line = tx.prepare_cached(
      "INSERT INTO nec_lines
         (user_id, design_id, pos, wire_a, at_a, wire_b, at_b, z0, length_m, velocity_factor)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;

    for (pos, design) in self.0.designs.iter().enumerate() {
      let SavedDesign {
        id,
        name,
        saved_at,
        model,
      } = design;
      let SavedModel {
        wires,
        feeds,
        loads,
        freq_hz,
        ground,
        conductivity,
        lines,
      } = model;
      let did = id.cast_signed();
      let (kind, eps_r, sigma) = ground_cols(ground);
      head.execute(params![
        user,
        did,
        pos as i64,
        name,
        saved_at,
        freq_hz,
        kind,
        eps_r,
        sigma,
        conductivity
      ])?;
      for (i, w) in wires.iter().enumerate() {
        let SavedWire {
          a,
          b,
          radius,
          segments,
        } = w;
        wire.execute(params![
          user,
          did,
          i as i64,
          a[0],
          a[1],
          a[2],
          b[0],
          b[1],
          b[2],
          radius,
          db_usize(*segments)
        ])?;
      }
      for (i, f) in feeds.iter().enumerate() {
        let SavedFeed { wire: w, at, volts } = f;
        feed.execute(params![user, did, i as i64, db_usize(*w), at, volts])?;
      }
      for (i, l) in loads.iter().enumerate() {
        let SavedLoad {
          wire: w,
          at,
          r_ohm,
          l_uh,
          c_pf,
        } = l;
        load.execute(params![
          user,
          did,
          i as i64,
          db_usize(*w),
          at,
          r_ohm,
          l_uh,
          c_pf
        ])?;
      }
      for (i, l) in lines.iter().enumerate() {
        let SavedLine {
          wire_a,
          at_a,
          wire_b,
          at_b,
          z0,
          length_m,
          velocity_factor,
        } = l;
        line.execute(params![
          user,
          did,
          i as i64,
          db_usize(*wire_a),
          at_a,
          db_usize(*wire_b),
          at_b,
          z0,
          length_m,
          velocity_factor
        ])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let mut wires = group(
      conn,
      user,
      "SELECT design_id, ax, ay, az, bx, by, bz, radius, segments
       FROM nec_wires WHERE user_id = ?1 ORDER BY design_id, pos",
      |r| {
        Ok(SavedWire {
          a: [r.get(1)?, r.get(2)?, r.get(3)?],
          b: [r.get(4)?, r.get(5)?, r.get(6)?],
          radius: r.get(7)?,
          segments: rust_usize(r.get(8)?),
        })
      },
    )?;
    let mut feeds = group(
      conn,
      user,
      "SELECT design_id, wire, at, volts FROM nec_feeds WHERE user_id = ?1 ORDER BY design_id, pos",
      |r| {
        Ok(SavedFeed {
          wire: rust_usize(r.get(1)?),
          at: r.get(2)?,
          volts: r.get(3)?,
        })
      },
    )?;
    let mut loads = group(
      conn,
      user,
      "SELECT design_id, wire, at, r_ohm, l_uh, c_pf
       FROM nec_loads WHERE user_id = ?1 ORDER BY design_id, pos",
      |r| {
        Ok(SavedLoad {
          wire: rust_usize(r.get(1)?),
          at: r.get(2)?,
          r_ohm: r.get(3)?,
          l_uh: r.get(4)?,
          c_pf: r.get(5)?,
        })
      },
    )?;
    let mut lines = group(
      conn,
      user,
      "SELECT design_id, wire_a, at_a, wire_b, at_b, z0, length_m, velocity_factor
       FROM nec_lines WHERE user_id = ?1 ORDER BY design_id, pos",
      |r| {
        Ok(SavedLine {
          wire_a: rust_usize(r.get(1)?),
          at_a: r.get(2)?,
          wire_b: rust_usize(r.get(3)?),
          at_b: r.get(4)?,
          z0: r.get(5)?,
          length_m: r.get(6)?,
          velocity_factor: r.get(7)?,
        })
      },
    )?;

    let heads: Vec<DesignRow> = conn
      .prepare_cached(
        "SELECT id, name, saved_at, freq_hz, ground, eps_r, sigma, conductivity
         FROM nec_designs WHERE user_id = ?1 ORDER BY pos",
      )?
      .query_map([user], |r| {
        Ok(DesignRow {
          id: r.get(0)?,
          name: r.get(1)?,
          saved_at: r.get(2)?,
          freq_hz: r.get(3)?,
          ground: r.get(4)?,
          eps_r: r.get(5)?,
          sigma: r.get(6)?,
          conductivity: r.get(7)?,
        })
      })?
      .collect::<rusqlite::Result<_>>()?;

    let designs = heads
      .into_iter()
      .map(|row| {
        Ok(SavedDesign {
          id: row.id.cast_unsigned(),
          name: row.name,
          saved_at: row.saved_at,
          model: SavedModel {
            wires: wires.remove(&row.id).unwrap_or_default(),
            feeds: feeds.remove(&row.id).unwrap_or_default(),
            loads: loads.remove(&row.id).unwrap_or_default(),
            freq_hz: row.freq_hz,
            ground: ground_of(&row.ground, row.eps_r, row.sigma)?,
            conductivity: row.conductivity,
            lines: lines.remove(&row.id).unwrap_or_default(),
          },
        })
      })
      .collect::<rusqlite::Result<_>>()?;
    // 走到这里还留在表里的子行，说明它们挂不上任何头记录：库被改坏了。
    // 静默变成空数组会让用户的设计少几根导线 / 几个负载，而且再也看不出来。
    if let Some(id) = wires
      .keys()
      .chain(feeds.keys())
      .chain(loads.keys())
      .chain(lines.keys())
      .next()
    {
      return Err(corrupt(format!("设计 {id} 的子行没有对应的头记录")));
    }
    Ok(Self(DesignBook { designs }))
  }
}
