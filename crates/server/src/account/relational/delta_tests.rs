//! 按 key 增量写入的测试：每个 key 独立核对「你看到的是不是现在这一版」，整批要么全生效要么全不生效。

use ham_web_core::account::Change;
use ham_web_core::delta::{Digest, manifest_fingerprint, manifest_of};
use ham_web_core::logbook::{LogEntry, Logbook};

use super::tests::{Map, entry, map, open, round_trip};
use super::{
  Delta, DeltaOutcome, DeltaRefused, apply_delta, fingerprint, load_some, manifest, total_bytes,
};
use crate::account::store::Store;

const BIG: u64 = 64 * 1024 * 1024;

fn put(key: &str, base: Option<Digest>, value: &str) -> Change {
  Change {
    key: key.to_owned(),
    base,
    value: Some(value.to_owned()),
  }
}

fn del(key: &str, base: Option<Digest>) -> Change {
  Change {
    key: key.to_owned(),
    base,
    value: None,
  }
}

fn d(text: &str) -> Option<Digest> {
  Some(Digest::of(text))
}

/// 一次增量写入（独立事务）。非 `Applied` 时事务不提交。
fn push(store: &Store, user: i64, changes: Vec<Change>) -> DeltaOutcome {
  push_capped(store, user, changes, BIG)
}

fn push_capped(store: &Store, user: i64, changes: Vec<Change>, cap: u64) -> DeltaOutcome {
  let delta = Delta::from_changes(changes).expect("测试里同一个 key 不会出现两次");
  store
    .with(|c| {
      let tx = c.transaction()?;
      let outcome = apply_delta(&tx, user, &delta, cap, i64::MAX)?;
      if outcome == DeltaOutcome::Applied {
        tx.commit()?;
      }
      Ok(outcome)
    })
    .unwrap()
}

fn current(store: &Store, user: i64) -> Map {
  store
    .with(|c| super::load(c, user))
    .unwrap()
    .into_map()
    .unwrap()
}

fn count(store: &Store, table: &str, user: i64) -> i64 {
  store
    .with(|c| {
      c.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE user_id = ?1"),
        [user],
        |r| r.get(0),
      )
    })
    .unwrap()
}

#[test]
fn new_keys_are_added_with_an_absent_base_and_show_up_in_the_manifest() {
  let (store, user) = open();
  let outcome = push(
    &store,
    user,
    vec![
      put("bookmarks", None, r#"["A-1","B-2"]"#),
      put("theme-ish", None, "plain text"),
    ],
  );
  assert_eq!(outcome, DeltaOutcome::Applied);
  assert_eq!(
    current(&store, user),
    map(&[
      ("bookmarks", r#"["A-1","B-2"]"#),
      ("theme-ish", "plain text")
    ])
  );
  // 专表里是行，清单里是摘要。
  assert_eq!(count(&store, "bookmarks", user), 2);
  let m = store.with(|c| manifest(c, user)).unwrap();
  assert_eq!(m["bookmarks"], Digest::of(r#"["A-1","B-2"]"#));
  assert_eq!(m["theme-ish"], Digest::of("plain text"));
}

#[test]
fn each_key_is_checked_against_the_version_the_caller_saw() {
  let (store, user) = open();
  push(&store, user, vec![put("a", None, "1"), put("b", None, "1")]);

  // 设备 A 改 a、设备 B 改 b：各自带着自己看到的摘要，互不影响。
  assert_eq!(
    push(&store, user, vec![put("a", d("1"), "2")]),
    DeltaOutcome::Applied
  );
  assert_eq!(
    push(&store, user, vec![put("b", d("1"), "9")]),
    DeltaOutcome::Applied
  );
  assert_eq!(current(&store, user), map(&[("a", "2"), ("b", "9")]));

  // 设备 B 再改 a，但它看到的还是 "1"：别处已经改成了 "2"，被挡下。
  assert_eq!(
    push(&store, user, vec![put("a", d("1"), "mine")]),
    DeltaOutcome::Conflict
  );
  assert_eq!(current(&store, user), map(&[("a", "2"), ("b", "9")]));
}

#[test]
fn a_batch_with_one_stale_key_changes_nothing_at_all() {
  let (store, user) = open();
  push(&store, user, vec![put("a", None, "1"), put("b", None, "1")]);
  let before = current(&store, user);
  // a 是对的，b 是过期的，c 是新增。整批都不生效，包括那个本来没问题的 a 与 c。
  let outcome = push(
    &store,
    user,
    vec![
      put("a", d("1"), "new"),
      put("b", d("stale"), "new"),
      put("c", None, "x"),
    ],
  );
  assert_eq!(outcome, DeltaOutcome::Conflict);
  assert_eq!(current(&store, user), before);
  assert_eq!(count(&store, "user_kv", user), 2);
}

#[test]
fn the_absent_and_present_expectations_are_both_enforced() {
  let (store, user) = open();
  push(&store, user, vec![put("a", None, "1")]);
  // 以为没有（None）其实已经有了：别处抢先新增了同名 key。
  assert_eq!(
    push(&store, user, vec![put("a", None, "2")]),
    DeltaOutcome::Conflict
  );
  // 以为有其实没有：别处已经删了它。
  assert_eq!(
    push(&store, user, vec![put("ghost", d("x"), "2")]),
    DeltaOutcome::Conflict
  );
  assert_eq!(
    push(&store, user, vec![del("ghost", d("x"))]),
    DeltaOutcome::Conflict
  );
  assert_eq!(current(&store, user), map(&[("a", "1")]));
}

#[test]
fn deleting_a_key_removes_its_table_rows_but_only_with_the_right_base() {
  let (store, user) = open();
  push(
    &store,
    user,
    vec![
      put("bookmarks", None, r#"["A-1","B-2","C-3"]"#),
      put("keep", None, "k"),
    ],
  );
  assert_eq!(count(&store, "bookmarks", user), 3);

  // 删的时候看到的是过期的一版：不删。
  assert_eq!(
    push(&store, user, vec![del("bookmarks", d("old"))]),
    DeltaOutcome::Conflict
  );
  assert_eq!(count(&store, "bookmarks", user), 3);

  let seen = d(r#"["A-1","B-2","C-3"]"#);
  assert_eq!(
    push(&store, user, vec![del("bookmarks", seen)]),
    DeltaOutcome::Applied
  );
  assert_eq!(count(&store, "bookmarks", user), 0, "专表里的行一起没了");
  assert_eq!(current(&store, user), map(&[("keep", "k")]));
  assert!(
    !store
      .with(|c| manifest(c, user))
      .unwrap()
      .contains_key("bookmarks")
  );
}

#[test]
fn a_key_can_move_between_a_table_and_the_catalog_through_deltas() {
  let (store, user) = open();
  push(&store, user, vec![put("bookmarks", None, r#"["A-1"]"#)]);
  assert_eq!(count(&store, "bookmarks", user), 1);
  // 改成解析不了的内容：退回目录里原样存放，专表里的行清掉。
  assert_eq!(
    push(
      &store,
      user,
      vec![put("bookmarks", d(r#"["A-1"]"#), "{坏了")]
    ),
    DeltaOutcome::Applied
  );
  assert_eq!(count(&store, "bookmarks", user), 0);
  assert_eq!(current(&store, user)["bookmarks"], "{坏了");
  // 再改回合法的：又进专表。
  assert_eq!(
    push(
      &store,
      user,
      vec![put("bookmarks", d("{坏了"), r#"["Z-9"]"#)]
    ),
    DeltaOutcome::Applied
  );
  assert_eq!(count(&store, "bookmarks", user), 1);
}

#[test]
fn the_manifest_digest_is_that_of_the_canonical_text_and_unchanged_keys_are_not_rewritten() {
  let (store, user) = open();
  // 缺字段的旧写法：服务端补成当前形态，摘要对规范文本算（不是上传原文）。
  let raw = r#"{"answered":3}"#;
  push(&store, user, vec![put("study-stats", None, raw)]);
  let canonical = current(&store, user)["study-stats"].clone();
  assert_ne!(canonical, raw);
  let m = store.with(|c| manifest(c, user)).unwrap();
  assert_eq!(
    m["study-stats"],
    Digest::of(&canonical),
    "摘要不是上传原文的摘要"
  );

  // 规范形态再传一遍：不重写（行没有被删了重建）。
  let rowid = |s: &Store| -> i64 {
    s.with(|c| {
      c.query_row("SELECT rowid FROM study_scopes WHERE bank = ''", [], |r| {
        r.get(0)
      })
    })
    .unwrap()
  };
  let before = rowid(&store);
  let again = push(
    &store,
    user,
    vec![put("study-stats", d(&canonical), &canonical)],
  );
  assert_eq!(again, DeltaOutcome::Applied);
  assert_eq!(rowid(&store), before, "没变的 key 不该被删了重写");
}

#[test]
fn a_delta_that_would_push_the_account_over_the_cap_is_refused_whole() {
  let (store, user) = open();
  push(&store, user, vec![put("a", None, "0123456789")]);
  let before = current(&store, user);
  // 上限 25 字节：已有 10，再加 10 + 10 就超了。
  let refused = push_capped(
    &store,
    user,
    vec![put("b", None, "0123456789"), put("c", None, "0123456789")],
    25,
  );
  assert_eq!(refused, DeltaOutcome::TooLarge);
  assert_eq!(current(&store, user), before, "超限的一批一个都不留");
  // 删掉旧的、再加新的，净增量不超就行。
  let fits = push_capped(
    &store,
    user,
    vec![del("a", d("0123456789")), put("b", None, "0123456789")],
    25,
  );
  assert_eq!(fits, DeltaOutcome::Applied);
  assert_eq!(store.with(|c| total_bytes(c, user)).unwrap(), 10);
}

#[test]
fn deltas_and_a_whole_replace_agree_on_the_manifest_and_the_fingerprint() {
  // 同样的最终内容，不论是一次整份写进去，还是一个个 key 增量攒出来，清单与指纹必须一样：
  // 指纹是 `If-Match` 的比较对象，整份接口和增量接口共用同一个版本号。
  let target = map(&[
    ("bookmarks", r#"["A-1","B-2"]"#),
    (
      "countdowns",
      r#"{"events":[{"id":1,"title":"考","target_ms":9}]}"#,
    ),
    ("plain", "x"),
    ("theme", "dark"),
  ]);
  let (whole, u1) = open();
  round_trip(&whole, u1, &target);

  let (stepwise, u2) = open();
  for (k, v) in &target {
    assert_eq!(
      push(&stepwise, u2, vec![put(k, None, v)]),
      DeltaOutcome::Applied
    );
  }
  let (m1, m2) = (
    whole.with(|c| manifest(c, u1)).unwrap(),
    stepwise.with(|c| manifest(c, u2)).unwrap(),
  );
  assert_eq!(m1, m2);
  assert_eq!(m1, manifest_of(&target, true));
  assert_eq!(
    whole.with(|c| fingerprint(c, u1)).unwrap(),
    stepwise.with(|c| fingerprint(c, u2)).unwrap()
  );
  assert_eq!(
    whole.with(|c| fingerprint(c, u1)).unwrap(),
    manifest_fingerprint(&manifest_of(&target, true))
  );
  // 设备专属的 key 照样存了，只是不在清单里。
  assert_eq!(current(&stepwise, u2)["theme"], "dark");
  assert!(!m2.contains_key("theme"));
}

#[test]
fn pulling_keys_returns_only_what_was_asked_for_in_canonical_form() {
  let (store, user) = open();
  round_trip(
    &store,
    user,
    &map(&[
      ("bookmarks", r#"["A-1"]"#),
      ("study-stats", r#"{"answered":3}"#),
      ("plain", "x"),
    ]),
  );
  let got = store
    .with(|c| {
      load_some(
        c,
        user,
        &[
          "study-stats".to_owned(),
          "plain".to_owned(),
          "nope".to_owned(),
        ],
      )
    })
    .unwrap()
    .into_map()
    .unwrap();
  assert_eq!(
    got.keys().map(String::as_str).collect::<Vec<_>>(),
    ["plain", "study-stats"]
  );
  assert!(
    got["study-stats"].contains("\"categories\""),
    "取回的是规范形态"
  );
}

#[test]
fn accounts_do_not_see_or_conflict_with_each_other() {
  let (store, a) = open();
  let b = store.create_user("bob", "h", 0).unwrap().id as i64;
  push(&store, a, vec![put("k", None, "alice")]);
  // bob 看不到 alice 的 k：他以为没有（None）是对的，写进去互不影响。
  assert_eq!(
    push(&store, b, vec![put("k", None, "bob")]),
    DeltaOutcome::Applied
  );
  assert_eq!(current(&store, a)["k"], "alice");
  assert_eq!(current(&store, b)["k"], "bob");
  assert_eq!(
    store.with(|c| manifest(c, a)).unwrap()["k"],
    Digest::of("alice")
  );
}

// ───────────────────────── 重复 key、拒绝时不动手 ─────────────────────────

/// 跑 `f` 期间，这条连接一共改了多少行（INSERT / UPDATE / DELETE，回滚了的也算）。
fn rows_touched_by(store: &Store, f: impl FnOnce()) -> u64 {
  let before = store.with(|c| Ok(c.total_changes())).unwrap();
  f();
  store.with(|c| Ok(c.total_changes())).unwrap() - before
}

#[test]
fn a_delta_naming_the_same_key_twice_is_refused_whole() {
  // 基线核对与写入都用事务开头的快照，第二条看不到第一条写了什么：不收。
  assert!(
    Delta::from_changes(vec![
      put("bookmarks", None, r#"["a"]"#),
      del("bookmarks", None)
    ])
    .is_none()
  );
  assert!(Delta::from_changes(vec![put("a", None, "1"), put("a", None, "2")]).is_none());
  assert!(Delta::from_changes(vec![put("a", None, "1"), put("b", None, "1")]).is_some());
}

#[test]
fn a_delta_over_the_cap_writes_nothing() {
  let (store, user) = open();
  push(&store, user, vec![put("a", None, "0123456789")]);
  // 超限要在动手写之前就判出来：事务回滚固然能收场，但白写一遍最多 64 MiB 也不该。
  let touched = rows_touched_by(&store, || {
    let refused = push_capped(
      &store,
      user,
      vec![put("b", None, "0123456789"), put("c", None, "0123456789")],
      25,
    );
    assert_eq!(refused, DeltaOutcome::TooLarge);
  });
  assert_eq!(touched, 0, "拒绝之前不该写任何一行");
  // 预估对「改写已有的 key」也按净增量算：10 字节换成 14 字节，总量 14 ≤ 15。
  let grown = push_capped(
    &store,
    user,
    vec![put("a", d("0123456789"), "0123456789abcd")],
    15,
  );
  assert_eq!(grown, DeltaOutcome::Applied);
}

#[test]
fn a_delta_whose_normalized_content_is_over_the_budget_is_refused_before_it_is_kept() {
  // 预算按**规范化之后**的字节算，边拆边数：第二个 key 一放不下就停，不会把全部都展开了再判。
  let one = entry(1, "JA1AAA");
  let raw = serde_json::to_string(&Logbook { entries: vec![one] }).unwrap();
  let canonical = Delta::from_changes_within(vec![put("logbook", None, &raw)], u64::MAX).is_ok();
  assert!(canonical);
  // 日志规范化之后远大于一个短 key：用这个差距造「原文很小、规范文本超预算」。
  // 能被 `LogEntry` 接受的最短写法（只给必填字段）：缺了必填字段会解码失败、退回原样存放，就看不出放大。
  let tiny =
    r#"{"entries":[{"callsign":"","date":"","freq":"","id":1,"mode":"","remark":"","time":""}]}"#;
  assert!(tiny.len() < 100);
  assert!(
    Delta::from_changes_within(vec![put("logbook", None, tiny)], 100).is_err(),
    "原文 {} 字节，规范化后远超 100",
    tiny.len()
  );
  assert_eq!(
    Delta::from_changes_within(vec![put("logbook", None, tiny)], 100).err(),
    Some(DeltaRefused::TooLarge)
  );
  // 删除不占预算；放得下就收。
  assert!(Delta::from_changes_within(vec![del("logbook", None)], 0).is_ok());
  assert!(Delta::from_changes_within(vec![put("a", None, "12345")], 5).is_ok());
  assert_eq!(
    Delta::from_changes_within(vec![put("a", None, "12345"), put("b", None, "6")], 5).err(),
    Some(DeltaRefused::TooLarge)
  );
  // 重复 key 与超预算是两种原因，处理函数据此回不同的状态码。
  assert_eq!(
    Delta::from_changes_within(vec![put("a", None, "1"), put("a", None, "2")], u64::MAX).err(),
    Some(DeltaRefused::DuplicateKey)
  );
}

#[test]
fn a_value_too_big_to_expand_is_kept_as_is_and_comes_back_byte_for_byte() {
  use super::{parse_within, qsos::Qsos};
  // 展开上限之上的原文不进专表：解码出来的结构体比原文大约 10 倍，是内存放大的来源。
  let raw = serde_json::to_string(&Logbook {
    entries: (1..=50).map(|id| entry(id, "JA1AAA")).collect(),
  })
  .unwrap();
  assert!(
    parse_within::<Qsos>("logbook", &raw, raw.len()).is_some(),
    "刚好放得下：展开"
  );
  assert!(
    parse_within::<Qsos>("logbook", &raw, raw.len() - 1).is_none(),
    "多一个字节：不展开"
  );

  // 不展开不等于丢：原样存放、原样取回，仍然同步。
  let (store, user) = open();
  let blob = "x".repeat(2048);
  let m = map(&[("logbook", &blob)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(count(&store, "qsos", user), 0, "解析不了的日志不进专表");
}

// ───────────────────────── 通联日志按行改 ─────────────────────────

fn book(entries: &[LogEntry]) -> String {
  serde_json::to_string(&Logbook {
    entries: entries.to_vec(),
  })
  .unwrap()
}

/// 把日志从 `before` 改成 `after`（带着上一版的摘要当基线），并核对读回来与 `after` 逐字节一致。
fn edit_book(store: &Store, user: i64, before: &[LogEntry], after: &[LogEntry]) {
  let base = (!before.is_empty()).then(|| Digest::of(&book(before)));
  let outcome = push(store, user, vec![put("logbook", base, &book(after))]);
  assert_eq!(outcome, DeltaOutcome::Applied);
  assert_eq!(current(store, user)["logbook"], book(after));
  assert_eq!(count(store, "qsos", user), after.len() as i64);
}

#[test]
fn changing_one_logbook_entry_rewrites_only_that_row() {
  let (store, user) = open();
  let mut entries: Vec<LogEntry> = (1..=300).map(|id| entry(id, &format!("JA{id}"))).collect();
  edit_book(&store, user, &[], &entries);

  let before = entries.clone();
  entries[150].remark = "改过".into();
  let touched = rows_touched_by(&store, || edit_book(&store, user, &before, &entries));
  // 删一行、插一行、目录记一笔；整表重写是 300 + 300 + 1。
  assert!(touched <= 5, "只改了一条通联，却动了 {touched} 行");

  // 什么都没变：一行都不动（目录那一行摘要相同也直接跳过）。
  let untouched = rows_touched_by(&store, || edit_book(&store, user, &entries, &entries));
  assert_eq!(untouched, 0);
}

#[test]
fn editing_a_logbook_in_place_reads_back_exactly() {
  let (store, user) = open();
  let mut entries: Vec<LogEntry> = (1..=40).map(|id| entry(id, &format!("JA{id}"))).collect();
  edit_book(&store, user, &[], &entries);

  // 删掉中间一条：它后面的行只是位置前移。
  let before = entries.clone();
  entries.remove(10);
  edit_book(&store, user, &before, &entries);

  // 在中间插一条新的。
  let before = entries.clone();
  entries.insert(20, entry(900, "W1AW"));
  edit_book(&store, user, &before, &entries);

  // 交换两条的先后顺序：内容没变，只有位置变。
  let before = entries.clone();
  entries.swap(3, 30);
  edit_book(&store, user, &before, &entries);

  // 同一次里又改、又删、又加。
  let before = entries.clone();
  entries[5].callsign = "BG4XXX".into();
  entries.remove(0);
  entries.push(entry(901, "K1ABC"));
  edit_book(&store, user, &before, &entries);

  // 清到只剩一条。
  let before = entries.clone();
  entries.truncate(1);
  edit_book(&store, user, &before, &entries);
}
