//! 多表存储的测试：核心是「拆开再拼回，逐字节不变」，以及数据确实落进了各张表。

use std::collections::BTreeMap;

use ham_web_core::logbook::{LogEntry, Logbook};
use ham_web_core::qsl_status::QslVia;
use rusqlite::Connection;

use super::{Split, apply, clear_all, fingerprint, load, manifest};
use crate::account::store::Store;

pub(super) type Map = BTreeMap<String, String>;

pub(super) fn map(pairs: &[(&str, &str)]) -> Map {
  pairs
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect()
}

/// 一条完整的通联。用真实类型生成：规范字节必须出自 `LogEntry` 自己的序列化
/// （`json!` 的键序是字母序，与结构体的字段序不同，拿它当夹具会让「无损入表」的判断永远落空）。
pub(super) fn entry(id: u64, call: &str) -> LogEntry {
  LogEntry {
    id,
    station_id: 1,
    date: "2026-01-02".into(),
    time: "03:04".into(),
    freq: "14.074".into(),
    mode: "FT8".into(),
    callsign: call.into(),
    rst_sent: "-10".into(),
    rst_rcvd: "-12".into(),
    tx_pwr: "50".into(),
    gridsquare: "PM01".into(),
    remark: "备注 \"引号\"".into(),
    qsl_sent: true,
    qsl_sent_via: QslVia::Bureau,
    ..LogEntry::default()
  }
}

fn logbook(calls: &[&str]) -> String {
  let entries = calls
    .iter()
    .enumerate()
    .map(|(i, c)| entry(i as u64 + 1, c))
    .collect();
  serde_json::to_string(&Logbook { entries }).unwrap()
}

const STATIONS: &str = r#"{"next_id":3,"profiles":[{"id":1,"label":"","callsign":"BG4XXX","operator":"","gridsquare":"PM01","rig":"","antenna":""},{"id":2,"label":"POTA","callsign":"BG4XXX/P","operator":"","gridsquare":"OM89","rig":"","antenna":""}],"active_id":1}"#;
const EXAMS: &str = r#"[{"bank":"A","correct":30,"total":40,"timestamp":1700000000000},{"bank":"B","correct":20,"total":60,"timestamp":1700000100000,"weak":true}]"#;

pub(super) fn open() -> (Store, i64) {
  let store = Store::open_in_memory().unwrap();
  let user = store.create_user("alice", "h", 0).unwrap();
  (store, user.id as i64)
}

fn tables(conn: &Connection, user: i64) -> BTreeMap<&'static str, i64> {
  [
    "qsos",
    "station_profiles",
    "station_books",
    "mistakes",
    "exam_results",
    "bookmarks",
    "dxcc_wanted_done",
    "user_kv",
  ]
  .into_iter()
  .map(|t| {
    let n = conn
      .query_row(
        &format!("SELECT COUNT(*) FROM {t} WHERE user_id = ?1"),
        [user],
        |r| r.get(0),
      )
      .unwrap();
    (t, n)
  })
  .collect()
}

/// 写进库再读出来，必须与原来的 map 完全一致。
pub(super) fn round_trip(store: &Store, user: i64, m: &Map) -> Map {
  store
    .with(|c| {
      let tx = c.transaction()?;
      apply(&tx, user, &Split::from_map(m.clone()))?;
      tx.commit()?;
      load(c, user)
    })
    .unwrap()
    .into_map()
    .unwrap()
}

#[test]
fn canonical_and_opaque_values_come_back_byte_for_byte() {
  // 规范形态的核心数据，加上不是 JSON / 没有专表 / 解析不了的内容：一个字节都不变。
  let m = map(&[
    ("logbook", &logbook(&["JA1AAA", "W1AW"])),
    ("station-book", STATIONS),
    ("exam-history", EXAMS),
    ("bookmarks", r#"["A-1","B-2","A-1"]"#),
    ("dxcc_wanted_done", "[]"),
    ("theme", "dark"),
    ("practice:2025-10:A", r#"{"index":3}"#),
    ("nec-designs-corrupt", "{半截 JSON"),
    ("empty", ""),
    ("emoji", "📡\u{0}\u{1f}\"\\"),
  ]);
  let (store, user) = open();
  assert_eq!(round_trip(&store, user, &m), m);
}

#[test]
fn a_valid_but_non_canonical_value_is_imported_into_the_tables_in_todays_shape() {
  // 老版本 / 手改过的日志：旧字段名、缺字段、多余空白、已不认识的字段。内容有效，所以进表；
  // 取回的是当前形态——旧字段名认成新字段，缺的补默认值，不认识的字段丢弃。
  let old = r#"{ "entries": [ {"id":1,"date":"2026-01-02","time":"03:04","freq":"7","mode":"CW","callsign":"JA1AAA","remark":"","sota_ref":"JA/TK-001","retired":1} ] }"#;
  let m = map(&[("logbook", old)]);
  let (store, user) = open();
  let back = round_trip(&store, user, &m);
  assert_eq!(store.with(|c| Ok(tables(c, user))).unwrap()["qsos"], 1);
  let book: Logbook = serde_json::from_str(&back["logbook"]).unwrap();
  assert_eq!(book.entries.len(), 1);
  assert_eq!(
    book.entries[0].my_sota_ref, "JA/TK-001",
    "旧字段名 sota_ref 要认成自己所在的山峰"
  );
  assert_eq!(book.entries[0].callsign, "JA1AAA");
  assert!(!back["logbook"].contains("retired"));
  // 取回的就是当前形态：它自己再导入一遍，一个字节都不再变（规范化是幂等的）。
  assert_eq!(round_trip(&store, user, &back), back);
}

#[test]
fn normalization_is_idempotent_and_the_manifest_matches_what_is_read_back() {
  // 前端靠这两条保持同步基线不漂：①取回的内容再上传，内容不变；
  // ②云端清单里每个 key 的摘要，等于「取回内容」按同一口径算出的摘要（所以本机能直接拿内容去比）。
  let (store, user) = open();
  let odd = map(&[
    (
      "logbook",
      r#"{"entries":[{"id":1,"date":"2026-1-2","time":"3:04","freq":"7","mode":"CW","callsign":"ja1aaa","remark":"","rst":"59"}]}"#,
    ),
    ("study-stats", r#"{"answered":3}"#),
    ("rpg-stars", r#"{"best":{"B":1,"A":2}}"#),
    (
      "question-stats",
      r#"{"items":{"k":{"attempts":1,"correct":1,"last_ms":1}}}"#,
    ),
    ("qsl-image:1", "data:image/JPG;base64,iVBORw0KGgo"),
    ("learning-plan", r#"{"bank":"A"}"#),
    ("theme", "dark"),
  ]);
  let once = round_trip(&store, user, &odd);
  let before = store.with(|c| manifest(c, user)).unwrap();
  assert_eq!(before, ham_web_core::delta::manifest_of(&once, true));
  assert!(!before.contains_key("theme"), "设备专属的 key 不在清单里");
  assert_eq!(
    store.with(|c| fingerprint(c, user)).unwrap(),
    ham_web_core::delta::manifest_fingerprint(&before)
  );
  let twice = round_trip(&store, user, &once);
  assert_eq!(once, twice);
  assert_eq!(store.with(|c| manifest(c, user)).unwrap(), before);
  // 设备专属的 key（theme）不进清单，但照样存、照样取回。
  assert_eq!(once["theme"], "dark");
}

#[test]
fn canonical_values_land_in_their_own_tables() {
  let (store, user) = open();
  let m = map(&[
    ("logbook", &logbook(&["JA1AAA", "W1AW", "BG4XXX"])),
    ("station-book", STATIONS),
    ("exam-history", EXAMS),
    ("bookmarks", r#"["A-1","B-2"]"#),
    ("theme", "dark"),
  ]);
  round_trip(&store, user, &m);
  let n = store.with(|c| Ok(tables(c, user))).unwrap();
  assert_eq!(n["qsos"], 3);
  assert_eq!(n["station_books"], 1);
  assert_eq!(n["station_profiles"], 2);
  assert_eq!(n["exam_results"], 2);
  assert_eq!(n["bookmarks"], 2);
  // 目录里每个 key 一行（含原样存放的 theme）。
  assert_eq!(n["user_kv"], 5);
  // 通联真的是按列存的：能直接用 SQL 查。
  let calls: Vec<String> = store
    .with(|c| {
      c.prepare(
        "SELECT callsign FROM qsos WHERE user_id = ?1 AND qsl_sent_via = 'bureau' ORDER BY pos",
      )?
      .query_map([user], |r| r.get(0))?
      .collect()
    })
    .unwrap();
  assert_eq!(calls, ["JA1AAA", "W1AW", "BG4XXX"]);
}

#[test]
fn an_empty_list_is_not_the_same_as_a_missing_key() {
  let (store, user) = open();
  let with_empty = map(&[("bookmarks", "[]")]);
  assert_eq!(round_trip(&store, user, &with_empty), with_empty);
  let without = Map::new();
  assert_eq!(round_trip(&store, user, &without), without);
}

#[test]
fn content_that_cannot_be_parsed_stays_verbatim_and_never_blocks_the_import() {
  let (store, user) = open();
  // 解析不了（不是 JSON、类型不对、缺必填字段）的内容也是用户数据：原样放在目录里，
  // 既不丢，也不拦住同一份备份里别的 key。
  let m = map(&[
    ("logbook", "{坏了"),
    ("bookmarks", r#"{"not":"a list"}"#),
    ("mistake-book", r#"{"records":[{"key":"k"}]}"#),
    ("exam-history", r#"[{"bank":"A"}]"#),
    ("dxcc_wanted_done", r#"["BY1"]"#),
  ]);
  assert_eq!(round_trip(&store, user, &m), m);
  let n = store.with(|c| Ok(tables(c, user))).unwrap();
  assert_eq!(
    (n["qsos"], n["bookmarks"], n["mistakes"], n["exam_results"]),
    (0, 0, 0, 0)
  );
  assert_eq!(n["dxcc_wanted_done"], 1, "能解析的 key 照常进表");
}

#[test]
fn duplicate_natural_keys_fall_back_to_verbatim_instead_of_failing() {
  let (store, user) = open();
  // 日志编号重复（手改 / 合并出的坏数据）：违反主键，整个 key 原样存放，而不是让整次同步失败。
  let dup = serde_json::to_string(&Logbook {
    entries: vec![entry(1, "A1AA"), entry(1, "B1BB")],
  })
  .unwrap();
  let m = map(&[("logbook", &dup)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(store.with(|c| Ok(tables(c, user))).unwrap()["qsos"], 0);
}

#[test]
fn replacing_a_backup_removes_keys_that_are_gone_and_moves_between_storages() {
  let (store, user) = open();
  let v1 = map(&[
    ("logbook", &logbook(&["JA1AAA", "W1AW"])),
    ("exam-history", EXAMS),
    ("theme", "dark"),
  ]);
  round_trip(&store, user, &v1);

  // 新备份：日志变少，成绩没了，theme 变了，新增一个 key。
  let v2 = map(&[
    ("logbook", &logbook(&["JA1AAA"])),
    ("theme", "light"),
    ("new", "1"),
  ]);
  assert_eq!(round_trip(&store, user, &v2), v2);
  let n = store.with(|c| Ok(tables(c, user))).unwrap();
  assert_eq!((n["qsos"], n["exam_results"], n["user_kv"]), (1, 0, 3));

  // 同一个 key 从「专表」变成「原样」再变回来：旧行不残留。
  let broken = map(&[("logbook", "{坏了")]);
  assert_eq!(round_trip(&store, user, &broken), broken);
  assert_eq!(store.with(|c| Ok(tables(c, user))).unwrap()["qsos"], 0);
  let healed = map(&[("logbook", &logbook(&["BG4XXX"]))]);
  assert_eq!(round_trip(&store, user, &healed), healed);
  assert_eq!(store.with(|c| Ok(tables(c, user))).unwrap()["qsos"], 1);
}

#[test]
fn unchanged_keys_are_not_rewritten() {
  let (store, user) = open();
  let m = map(&[("logbook", &logbook(&["JA1AAA"])), ("theme", "dark")]);
  round_trip(&store, user, &m);
  // 给通联行打个只有「被重写」才会消失的记号：同内容再同步一次，记号必须还在。
  store
    .with(|c| {
      c.execute("UPDATE qsos SET pos = 77 WHERE user_id = ?1", [user])
        .map(|_| ())
    })
    .unwrap();
  round_trip(&store, user, &m);
  let pos: i64 = store
    .with(|c| {
      c.query_row("SELECT pos FROM qsos WHERE user_id = ?1", [user], |r| {
        r.get(0)
      })
    })
    .unwrap();
  assert_eq!(pos, 77, "内容没变的 key 不该被删了重写");
  // 内容变了就要重写。
  let changed = map(&[("logbook", &logbook(&["W1AW"])), ("theme", "dark")]);
  round_trip(&store, user, &changed);
  let pos: i64 = store
    .with(|c| {
      c.query_row("SELECT pos FROM qsos WHERE user_id = ?1", [user], |r| {
        r.get(0)
      })
    })
    .unwrap();
  assert_eq!(pos, 0);
}

#[test]
fn accounts_are_isolated_and_clearing_one_leaves_the_other() {
  let store = Store::open_in_memory().unwrap();
  let a = store.create_user("alice", "h", 0).unwrap().id as i64;
  let b = store.create_user("bob", "h", 0).unwrap().id as i64;
  let ma = map(&[
    ("logbook", &logbook(&["JA1AAA"])),
    ("bookmarks", r#"["A-1"]"#),
  ]);
  let mb = map(&[("logbook", &logbook(&["W1AW", "K1ABC"]))]);
  assert_eq!(round_trip(&store, a, &ma), ma);
  assert_eq!(round_trip(&store, b, &mb), mb);
  store.with(|c| clear_all(c, a)).unwrap();
  let left = store.with(|c| Ok(tables(c, a))).unwrap();
  assert!(left.values().all(|n| *n == 0), "{left:?}");
  assert_eq!(round_trip(&store, b, &mb), mb);
}

#[test]
fn deleting_the_user_cascades_into_every_table() {
  let (store, user) = open();
  let m = map(&[
    ("logbook", &logbook(&["JA1AAA"])),
    ("station-book", STATIONS),
    ("exam-history", EXAMS),
    ("bookmarks", r#"["A-1"]"#),
    ("theme", "dark"),
  ]);
  round_trip(&store, user, &m);
  store.delete_user(user as u64).unwrap();
  let n = store.with(|c| Ok(tables(c, user))).unwrap();
  assert!(n.values().all(|n| *n == 0), "外键级联应清空所有表：{n:?}");
}

#[test]
fn from_map_within_stops_as_soon_as_the_budget_is_spent() {
  let m = map(&[
    ("a", "0123456789"),
    ("b", "0123456789"),
    ("c", "0123456789"),
  ]);
  // 总量按规范文本的字节数算；恰好等于预算收，多一个字节就丢。
  assert!(Split::from_map_within(m.clone(), 29).is_none());
  let fits = Split::from_map_within(m.clone(), 30).unwrap();
  assert_eq!(fits.total_bytes(), 30);
  assert_eq!(fits.into_map().unwrap(), m);

  // 规范化会补齐缺省字段：原文刚好在预算里，规范化之后超了，照样拒绝。
  let raw = r#"{"answered":3}"#;
  let one = map(&[("study-stats", raw)]);
  assert!(Split::from_map(one.clone()).total_bytes() > raw.len() as u64);
  assert!(Split::from_map_within(one, raw.len() as u64).is_none());
}
