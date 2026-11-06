//! `question-notes`：题目私人笔记（题目 stable_id → 笔记文本）。
//!
//! 前端用 `HashMap` 写出这个 JSON 对象，键序每次都可能不同，所以不能先解析成有序映射再比对——
//! 那样几乎每份都会因为「键序变了」被判成不规范。这里按**出现顺序**读成列表、按同一顺序写回（`pos`），
//! 不论前端写出什么顺序都能逐字节还原。同一个 id 出现两次（手改的坏数据）的整个 key 留在目录里原样存放。

use std::collections::HashSet;
use std::fmt;

use rusqlite::{Connection, Transaction, params};
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};

use super::{Key, Table};

/// 保持键序的 `{string: string}`。
struct OrderedNotes(Vec<(String, String)>);

impl<'de> Deserialize<'de> for OrderedNotes {
  fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
    struct V;
    impl<'de> Visitor<'de> for V {
      type Value = OrderedNotes;

      fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON object of string values")
      }

      fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OrderedNotes, A::Error> {
        let mut out = Vec::new();
        while let Some(entry) = map.next_entry::<String, String>()? {
          out.push(entry);
        }
        Ok(OrderedNotes(out))
      }
    }
    d.deserialize_map(V)
  }
}

pub(super) struct QuestionNotes(OrderedNotes);

impl Table for QuestionNotes {
  const KEY: Key = Key::Exact("question-notes");
  const TABLES: &'static [&'static str] = &["question_notes"];

  fn decode(_key: &str, raw: &str) -> Option<Self> {
    serde_json::from_str::<OrderedNotes>(raw).ok().map(Self)
  }

  fn encode(&self) -> Option<String> {
    let mut out = String::from("{");
    for (i, (id, note)) in self.0.0.iter().enumerate() {
      if i > 0 {
        out.push(',');
      }
      out.push_str(&serde_json::to_string(id).ok()?);
      out.push(':');
      out.push_str(&serde_json::to_string(note).ok()?);
    }
    out.push('}');
    Some(out)
  }

  fn storable(&self) -> bool {
    let mut seen = HashSet::new();
    self.0.0.iter().all(|(id, _)| seen.insert(id.as_str()))
  }

  fn insert(&self, tx: &Transaction<'_>, user: i64, _key: &str) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare_cached(
      "INSERT INTO question_notes (user_id, pos, stable_id, note) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (pos, (id, note)) in self.0.0.iter().enumerate() {
      stmt.execute(params![user, pos as i64, id, note])?;
    }
    Ok(())
  }

  fn load(conn: &Connection, user: i64, _key: &str) -> rusqlite::Result<Self> {
    let notes = conn
      .prepare_cached("SELECT stable_id, note FROM question_notes WHERE user_id = ?1 ORDER BY pos")?
      .query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<rusqlite::Result<_>>()?;
    Ok(Self(OrderedNotes(notes)))
  }
}
