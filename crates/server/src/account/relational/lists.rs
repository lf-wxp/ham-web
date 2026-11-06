//! 有序的值列表：每个 key 的内容是一个 JSON 数组，元素在表里一行一个、顺序用 `pos` 保留。
//!
//! 数组顺序在前端是有意义的（点击先后）；元素允许重复，所以主键是 `(user_id, pos)` 而不是元素本身。

use rusqlite::{Connection, Transaction};

use super::{Key, Table};

macro_rules! string_list {
  ($(#[$doc:meta])* $ty:ident, $key:literal, $table:literal) => {
    $(#[$doc])*
    pub(super) struct $ty(Vec<String>);

    impl Table for $ty {
      const KEY: Key = Key::Exact($key);
      const TABLES: &'static [&'static str] = &[$table];

      fn decode(_key: &str, raw: &str) -> Option<Self> {
        serde_json::from_str::<Vec<String>>(raw).ok().map(Self)
      }

      fn encode(&self) -> Option<String> {
        serde_json::to_string(&self.0).ok()
      }

      fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
        let mut stmt = tx.prepare_cached(concat!(
          "INSERT INTO ", $table, " (user_id, pos, value) VALUES (?1, ?2, ?3)"
        ))?;
        for (pos, value) in self.0.iter().enumerate() {
          stmt.execute(rusqlite::params![user, pos as i64, value])?;
        }
        Ok(())
      }

      fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
        conn
          .prepare_cached(concat!(
            "SELECT value FROM ", $table, " WHERE user_id = ?1 ORDER BY pos"
          ))?
          .query_map([user], |r| r.get(0))?
          .collect::<rusqlite::Result<_>>()
          .map(Self)
      }
    }
  };
}

string_list!(
  /// `bookmarks`：收藏题目的 stable_id。
  Bookmarks, "bookmarks", "bookmarks"
);
string_list!(
  /// `dxcc_wanted_done`：已通联的 DXCC 稀有前缀。
  DxccWantedDone, "dxcc_wanted_done", "dxcc_wanted_done"
);
string_list!(
  /// `achievements-seen`：已提示过的成就解锁 id。
  AchievementsSeen, "achievements-seen", "achievements_seen"
);

/// `qsl-image-index`：存了卡片影像的通联编号（`LogEntry.id`）。
pub(super) struct QslImageIndex(Vec<u64>);

impl Table for QslImageIndex {
  const KEY: Key = Key::Exact("qsl-image-index");
  const TABLES: &'static [&'static str] = &["qsl_image_index"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<Vec<u64>>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt =
      tx.prepare_cached("INSERT INTO qsl_image_index (user_id, pos, qso_id) VALUES (?1, ?2, ?3)")?;
    for (pos, id) in self.0.iter().enumerate() {
      stmt.execute(rusqlite::params![user, pos as i64, id.cast_signed()])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    conn
      .prepare_cached("SELECT qso_id FROM qsl_image_index WHERE user_id = ?1 ORDER BY pos")?
      .query_map([user], |r| Ok(r.get::<_, i64>(0)?.cast_unsigned()))?
      .collect::<rusqlite::Result<_>>()
      .map(Self)
  }
}
