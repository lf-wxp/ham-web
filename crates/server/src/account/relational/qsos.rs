//! `logbook`：通联日志。每条通联一行，四十个字段各占一列。
//!
//! 主键是 `(user_id, id)`：`id` 是日志自己的自增编号，同一账号内唯一才入表（重复的坏数据整个
//! key 退回原样存放）。条目的先后顺序用 `pos` 保留——前端按数组顺序展示，顺序也算数据。
//!
//! 字段与 `ham_web_core::logbook::LogEntry` 一一对应；读回时用不带 `..` 的结构体字面量，
//! 将来给 `LogEntry` 加字段而忘了在这里加列，会直接编译失败而不是悄悄丢数据。

use std::collections::{HashMap, HashSet};

use ham_web_core::logbook::{LogEntry, Logbook};
use ham_web_core::qsl_status::QslVia;
use rusqlite::{Connection, Row, Statement, Transaction, named_params, params};

use super::{Key, Table};

pub(super) struct Qsos(Logbook);

const INSERT: &str = "INSERT INTO qsos (
    user_id, id, pos, operator, station_id, date, time, time_off, freq, band, mode, callsign,
    rst, rst_sent, rst_rcvd, tx_pwr, gridsquare, name, qth, prop_mode, sat_name,
    my_sota_ref, my_pota_ref, contact_sota_ref, contact_pota_ref, contest_id, stx, srx,
    dxcc, cqz, ituz, state, iota, remark, qsl_sent, qsl_sent_via, qsl_rcvd,
    lotw_sent, lotw_rcvd, eqsl_sent, eqsl_rcvd, qrz_rcvd
  ) VALUES (
    :user_id, :id, :pos, :operator, :station_id, :date, :time, :time_off, :freq, :band, :mode,
    :callsign, :rst, :rst_sent, :rst_rcvd, :tx_pwr, :gridsquare, :name, :qth, :prop_mode,
    :sat_name, :my_sota_ref, :my_pota_ref, :contact_sota_ref, :contact_pota_ref, :contest_id,
    :stx, :srx, :dxcc, :cqz, :ituz, :state, :iota, :remark, :qsl_sent, :qsl_sent_via,
    :qsl_rcvd, :lotw_sent, :lotw_rcvd, :eqsl_sent, :eqsl_rcvd, :qrz_rcvd
  )";

const SELECT: &str = "SELECT
    id, operator, station_id, date, time, time_off, freq, band, mode, callsign,
    rst, rst_sent, rst_rcvd, tx_pwr, gridsquare, name, qth, prop_mode, sat_name,
    my_sota_ref, my_pota_ref, contact_sota_ref, contact_pota_ref, contest_id, stx, srx,
    dxcc, cqz, ituz, state, iota, remark, qsl_sent, qsl_sent_via, qsl_rcvd,
    lotw_sent, lotw_rcvd, eqsl_sent, eqsl_rcvd, qrz_rcvd
  FROM qsos WHERE user_id = ?1 ORDER BY pos";

fn from_row(r: &Row<'_>) -> rusqlite::Result<LogEntry> {
  Ok(LogEntry {
    id: r.get::<_, i64>("id")?.cast_unsigned(),
    operator: r.get("operator")?,
    station_id: r.get::<_, i64>("station_id")?.cast_unsigned(),
    date: r.get("date")?,
    time: r.get("time")?,
    time_off: r.get("time_off")?,
    freq: r.get("freq")?,
    band: r.get("band")?,
    mode: r.get("mode")?,
    callsign: r.get("callsign")?,
    rst: r.get("rst")?,
    rst_sent: r.get("rst_sent")?,
    rst_rcvd: r.get("rst_rcvd")?,
    tx_pwr: r.get("tx_pwr")?,
    gridsquare: r.get("gridsquare")?,
    name: r.get("name")?,
    qth: r.get("qth")?,
    prop_mode: r.get("prop_mode")?,
    sat_name: r.get("sat_name")?,
    my_sota_ref: r.get("my_sota_ref")?,
    my_pota_ref: r.get("my_pota_ref")?,
    contact_sota_ref: r.get("contact_sota_ref")?,
    contact_pota_ref: r.get("contact_pota_ref")?,
    contest_id: r.get("contest_id")?,
    stx: r.get("stx")?,
    srx: r.get("srx")?,
    dxcc: r.get("dxcc")?,
    cqz: r.get("cqz")?,
    ituz: r.get("ituz")?,
    state: r.get("state")?,
    iota: r.get("iota")?,
    remark: r.get("remark")?,
    qsl_sent: r.get("qsl_sent")?,
    qsl_sent_via: QslVia::from_key(&r.get::<_, String>("qsl_sent_via")?),
    qsl_rcvd: r.get("qsl_rcvd")?,
    lotw_sent: r.get("lotw_sent")?,
    lotw_rcvd: r.get("lotw_rcvd")?,
    eqsl_sent: r.get("eqsl_sent")?,
    eqsl_rcvd: r.get("eqsl_rcvd")?,
    qrz_rcvd: r.get("qrz_rcvd")?,
  })
}

/// 写一行。整表写入与按行改动共用它，列与 [`INSERT`] 一一对应。
fn insert_row(
  stmt: &mut Statement<'_>,
  user: i64,
  pos: usize,
  e: &LogEntry,
) -> rusqlite::Result<()> {
  stmt.execute(named_params! {
    ":user_id": user,
    ":id": e.id.cast_signed(),
    ":pos": pos as i64,
    ":operator": e.operator,
    ":station_id": e.station_id.cast_signed(),
    ":date": e.date,
    ":time": e.time,
    ":time_off": e.time_off,
    ":freq": e.freq,
    ":band": e.band,
    ":mode": e.mode,
    ":callsign": e.callsign,
    ":rst": e.rst,
    ":rst_sent": e.rst_sent,
    ":rst_rcvd": e.rst_rcvd,
    ":tx_pwr": e.tx_pwr,
    ":gridsquare": e.gridsquare,
    ":name": e.name,
    ":qth": e.qth,
    ":prop_mode": e.prop_mode,
    ":sat_name": e.sat_name,
    ":my_sota_ref": e.my_sota_ref,
    ":my_pota_ref": e.my_pota_ref,
    ":contact_sota_ref": e.contact_sota_ref,
    ":contact_pota_ref": e.contact_pota_ref,
    ":contest_id": e.contest_id,
    ":stx": e.stx,
    ":srx": e.srx,
    ":dxcc": e.dxcc,
    ":cqz": e.cqz,
    ":ituz": e.ituz,
    ":state": e.state,
    ":iota": e.iota,
    ":remark": e.remark,
    ":qsl_sent": e.qsl_sent,
    ":qsl_sent_via": e.qsl_sent_via.key(),
    ":qsl_rcvd": e.qsl_rcvd,
    ":lotw_sent": e.lotw_sent,
    ":lotw_rcvd": e.lotw_rcvd,
    ":eqsl_sent": e.eqsl_sent,
    ":eqsl_rcvd": e.eqsl_rcvd,
    ":qrz_rcvd": e.qrz_rcvd,
  })?;
  Ok(())
}

impl Table for Qsos {
  const KEY: Key = Key::Exact("logbook");
  const TABLES: &'static [&'static str] = &["qsos"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Logbook>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.entries.iter().all(|e| seen.insert(e.id))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(INSERT)?;
    for (pos, e) in self.0.entries.iter().enumerate() {
      insert_row(&mut stmt, user, pos, e)?;
    }
    Ok(())
  }

  /// 只动有变化的行：日志少则几百、多则几万条，通常一次只加一条或改一条，
  /// 不该为此把全部行删光重写（每行四十多列，还全程占着全局写锁）。
  ///
  /// 旧行读出来按编号对照：没了的删掉；内容没变只是位置变了的（删掉中间一条之后的尾部）只改 `pos`；
  /// 内容变了的删了重写（与整表重写同一套语义，不另造 `UPDATE` 的四十列写法）；新来的插入。
  /// 读回来与新内容不相等就一律当「变了」重写，所以读写之间有任何不对称，代价只是多写几行，不会写错。
  fn replace(&self, tx: &Transaction<'_>, user: i64, key: &str) -> rusqlite::Result<()> {
    let old = Self::load(tx, user, key)?.0.entries;
    let old_at: HashMap<u64, (usize, &LogEntry)> = old
      .iter()
      .enumerate()
      .map(|(pos, e)| (e.id, (pos, e)))
      .collect();
    let kept: HashSet<u64> = self.0.entries.iter().map(|e| e.id).collect();

    let mut delete = tx.prepare_cached("DELETE FROM qsos WHERE user_id = ?1 AND id = ?2")?;
    for e in &old {
      if !kept.contains(&e.id) {
        delete.execute(params![user, e.id.cast_signed()])?;
      }
    }
    let mut renumber =
      tx.prepare_cached("UPDATE qsos SET pos = ?3 WHERE user_id = ?1 AND id = ?2")?;
    let mut insert = tx.prepare_cached(INSERT)?;
    for (pos, e) in self.0.entries.iter().enumerate() {
      match old_at.get(&e.id) {
        Some(&(old_pos, was)) if was == e => {
          if old_pos != pos {
            renumber.execute(params![user, e.id.cast_signed(), pos as i64])?;
          }
        }
        Some(_) => {
          delete.execute(params![user, e.id.cast_signed()])?;
          insert_row(&mut insert, user, pos, e)?;
        }
        None => insert_row(&mut insert, user, pos, e)?,
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let entries = conn
      .prepare_cached(SELECT)?
      .query_map([user], from_row)?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(Logbook { entries }))
  }
}
