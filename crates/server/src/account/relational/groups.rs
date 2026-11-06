//! `bookmark-groups`：收藏分组。分组按 `pos` 保序，每个分组下的题目也按 `pos` 保序。
//!
//! 分组名在应用里创建 / 重命名时保证不重复，但库里不靠它做主键（手改过的数据可能重名）；
//! 题目挂在分组的位置上，删分组时级联删除。

use ham_web_core::bookmark_groups::{BookmarkGroup, BookmarkGroups};
use rusqlite::{Connection, Transaction, params};

use super::{Key, Table};

pub(super) struct BookmarkGroupsTable(BookmarkGroups);

impl Table for BookmarkGroupsTable {
  const KEY: Key = Key::Exact("bookmark-groups");
  const TABLES: &'static [&'static str] = &["bookmark_group_items", "bookmark_groups"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<BookmarkGroups>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    serde_json::to_string(&self.0).ok()
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let BookmarkGroups { groups } = &self.0;
    for (gpos, group) in groups.iter().enumerate() {
      let BookmarkGroup { name, ids } = group;
      tx.prepare_cached("INSERT INTO bookmark_groups (user_id, pos, name) VALUES (?1, ?2, ?3)")?
        .execute(params![user, gpos as i64, name])?;
      let mut stmt = tx.prepare_cached(
        "INSERT INTO bookmark_group_items (user_id, group_pos, pos, stable_id)
         VALUES (?1, ?2, ?3, ?4)",
      )?;
      for (pos, id) in ids.iter().enumerate() {
        stmt.execute(params![user, gpos as i64, pos as i64, id])?;
      }
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let heads: Vec<(i64, String)> = conn
      .prepare_cached("SELECT pos, name FROM bookmark_groups WHERE user_id = ?1 ORDER BY pos")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare_cached(
      "SELECT stable_id FROM bookmark_group_items
       WHERE user_id = ?1 AND group_pos = ?2 ORDER BY pos",
    )?;
    let mut groups = Vec::with_capacity(heads.len());
    for (gpos, name) in heads {
      let ids = stmt
        .query_map(params![user, gpos], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<_>>()?;
      groups.push(BookmarkGroup { name, ids });
    }
    Ok(Self(BookmarkGroups { groups }))
  }
}
