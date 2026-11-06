//! 每一张专表的「规范值进表、原样取回」测试。
//!
//! 夹具一律用**类型自己的序列化**生成：专表认的是类型写出的字节，手写的 JSON（键序、空白、缺省字段）
//! 不一定是。另一半是反面用例——类型还原不出原文字节的值必须原样留在目录里，一个字节都不丢。

use ham_web_core::bank::Bank;
use ham_web_core::bookmark_groups::BookmarkGroups;
use ham_web_core::card_review::Schedule;
use ham_web_core::cw_runner::RunnerStats;
use ham_web_core::daily_challenge::DailyResults;
use ham_web_core::dx_watch::AlertSettings;
use ham_web_core::exam_review::ExamReview;
use ham_web_core::koch::KochProgress;
use ham_web_core::mistake_book::{MistakeBook, MistakeRecord, StudyStats};
use ham_web_core::question::{Codes, Pages, QuestionItem, QuestionOption, QuestionType};
use ham_web_core::question_stats::QuestionStats;
use ham_web_core::rpg::{Bestiary, StageStars};
use ham_web_core::sat_watch::SatWatch;
use ham_web_core::saved_state::{ExamSavedState, PracticeSavedState};
use ham_web_core::study_plan::{DailyLog, StudyPlan};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::clear_all;
use super::tests::{Map, map, open, round_trip};
use crate::account::store::Store;

/// 把一段「字段齐全」的 JSON 过一遍类型，得到类型写出的规范文本。
pub(super) fn canon<T: Serialize + DeserializeOwned>(partial: &str) -> String {
  serde_json::to_string(&serde_json::from_str::<T>(partial).expect(partial)).unwrap()
}

/// 一个 key 的规范值、它该落进的表与行数。
pub(super) struct Case {
  pub(super) key: &'static str,
  pub(super) raw: String,
  pub(super) tables: &'static [(&'static str, i64)],
}

/// 全部专表的用例：核心类型的 + 页面小文档 / 工具器材 / NEC / 错题暂存的（见 `local_tests`）。
fn cases() -> Vec<Case> {
  let mut all = core_cases();
  all.extend(super::local_tests::cases());
  all
}

fn core_cases() -> Vec<Case> {
  vec![
    Case {
      key: "study-stats",
      raw: canon::<StudyStats>(
        r#"{"categories":{"1":{"answered":3,"correct":2}},"subs":{"1.1":{"answered":3,"correct":2},"1.2":{"answered":0,"correct":0}},"answered":3,"correct":2,"banks":{"A":{"categories":{"1":{"answered":3,"correct":2}},"subs":{},"answered":3,"correct":2,"seen":["aa","bb","cc"]},"C":{"answered":0,"correct":0,"seen":[]}}}"#,
      ),
      // 范围：全部、A、C；分项：全部 1+2 条、A 1 条。
      tables: &[("study_scopes", 3), ("study_tallies", 4), ("study_seen", 3)],
    },
    Case {
      key: "question-stats",
      raw: canon::<QuestionStats>(
        r#"{"items":{"k1":{"attempts":3,"correct":2,"last_ms":5,"duration_ms":900},"k2":{"attempts":1,"correct":0,"last_ms":6,"duration_ms":0}}}"#,
      ),
      tables: &[("question_stats", 2)],
    },
    Case {
      key: "study-daily",
      raw: canon::<DailyLog>(
        r#"{"days":{"2026-01-01":{"answered":5,"new":2,"duration_ms":1000},"2026-01-02":{"answered":1,"new":0,"duration_ms":9}}}"#,
      ),
      tables: &[("study_daily", 2)],
    },
    Case {
      key: "daily-challenge",
      raw: canon::<DailyResults>(r#"{"days":{"2026-01-01":{"correct":8,"total":10}}}"#),
      tables: &[("daily_challenge", 1)],
    },
    Case {
      key: "rpg-stars",
      raw: canon::<StageStars>(r#"{"best":{"A:safety":3,"B:rf":1}}"#),
      tables: &[("stage_stars", 2)],
    },
    Case {
      key: "rpg-bestiary",
      raw: canon::<Bestiary>(r#"{"defeated":["x","y","z"]}"#),
      tables: &[("bestiary", 3)],
    },
    Case {
      key: "card-review",
      raw: canon::<Schedule>(
        r#"{"cards":{"q:QRM":{"ease":2.5,"interval_days":1.0,"due_ms":10,"last_ms":5,"reps":2,"lapses":0},"q:QSL":{"ease":1.3,"interval_days":0.5,"due_ms":11,"last_ms":6,"reps":1,"lapses":1}},"new_day":"2026-01-01","new_today":{"qcode":2,"morse":1}}"#,
      ),
      tables: &[
        ("card_schedule", 1),
        ("card_states", 2),
        ("card_new_today", 2),
      ],
    },
    Case {
      key: "morse-koch",
      raw: canon::<KochProgress>(
        r#"{"level":3,"char_wpm":20,"effective_wpm":10,"chars":{"K":{"sent":5,"correct":4},"M":{"sent":2,"correct":2}},"level_tally":{"sent":1,"correct":1}}"#,
      ),
      tables: &[("koch_progress", 1), ("koch_chars", 2)],
    },
    Case {
      key: "morse-runner",
      raw: canon::<RunnerStats>(
        r#"{"wpm":22,"adaptive":true,"cut_numbers":false,"pileup":true,"sessions":3,"qsos":9,"calls_ok":8,"best_score":120,"top_wpm":30,"missed":{"K":2,"Q":1}}"#,
      ),
      tables: &[("runner_stats", 1), ("runner_missed", 2)],
    },
    Case {
      key: "bookmark-groups",
      raw: canon::<BookmarkGroups>(
        r#"{"groups":[{"name":"天线","ids":["A-1","A-2"]},{"name":"空","ids":[]}]}"#,
      ),
      tables: &[("bookmark_groups", 2), ("bookmark_group_items", 2)],
    },
    Case {
      key: "exam:last-review",
      raw: canon::<ExamReview>(
        r#"{"bank":"A","finished_at_ms":1,"items":[{"id":"A-1","code":"J1","question":"题","options":["A. 是","B. 否"],"answer":["A"],"given":[],"explanation":"","category":"1.1"}]}"#,
      ),
      tables: &[
        ("exam_reviews", 1),
        ("exam_review_items", 1),
        ("exam_review_options", 2),
        ("exam_review_choices", 1),
      ],
    },
    Case {
      key: "mistake-book",
      raw: mistake_book(),
      // 三条记录；选项 2 + 0 + 0；标准答案 1 + 我的作答 2；错过的题库 2。
      tables: &[
        ("mistakes", 3),
        ("mistake_options", 2),
        ("mistake_choices", 3),
        ("mistake_banks", 2),
      ],
    },
    Case {
      key: "learning-plan",
      raw: canon::<StudyPlan>(r#"{"target_date":"2026-12-01","bank":"A","reminder_time":"20:00"}"#),
      tables: &[("study_plan", 1)],
    },
    Case {
      key: "dx-alerts",
      raw: canon::<AlertSettings>(
        r#"{"new_dxcc":true,"new_band":false,"calls":["BY1*","JA1ABC"]}"#,
      ),
      tables: &[("dx_alerts", 1), ("dx_alert_calls", 2)],
    },
    Case {
      key: "sat-watch",
      raw: canon::<SatWatch>(
        r#"{"lat":39.9,"lon":116.4,"min_elev":10.0,"favorites":[{"norad":25544,"name":"ISS"}],"alerts":true,"lead_min":10,"apt_alert":false}"#,
      ),
      tables: &[("sat_watch", 1), ("sat_favorites", 1)],
    },
    Case {
      key: "practice:2025-10:A",
      raw: canon::<PracticeSavedState>(
        r#"{"version":2,"bank":"A","versionId":"2025-10","timestamp":1,"index":2,"order":"sequential","showAnswer":false,"orderIndices":[0,1,2],"answersByPosition":[["A"],null,["B","C"]],"total":3}"#,
      ),
      tables: &[("practice_progress", 1)],
    },
    Case {
      key: "exam:savedState:2025-10:A",
      raw: canon::<ExamSavedState>(
        r#"{"version":2,"bank":"A","versionId":"2025-10","timestamp":1,"endAtMs":99,"index":0,"answersByPosition":[null],"flagsByPosition":[false],"total":1,"questionIds":["A-1"]}"#,
      ),
      tables: &[("exam_progress", 1)],
    },
    Case {
      key: "qsl-image:7",
      raw: "data:image/png;base64,iVBORw0KGgo=".to_owned(),
      tables: &[("qsl_images", 1)],
    },
    Case {
      key: "qsl-image-index",
      raw: "[7,9]".to_owned(),
      tables: &[("qsl_image_index", 2)],
    },
    Case {
      key: "achievements-seen",
      raw: r#"["first","streak7"]"#.to_owned(),
      tables: &[("achievements_seen", 2)],
    },
    // 键序故意不是字典序：前端用 HashMap 写，顺序随机，必须按原顺序还原。
    Case {
      key: "question-notes",
      raw: r#"{"B-2":"笔记 \"x\"\n换行","A-1":"a"}"#.to_owned(),
      tables: &[("question_notes", 2)],
    },
  ]
}

pub(super) fn rows(store: &Store, table: &str, user: i64) -> i64 {
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

pub(super) fn storage_of(store: &Store, user: i64, key: &str) -> String {
  store
    .with(|c| {
      c.query_row(
        "SELECT storage FROM user_kv WHERE user_id = ?1 AND key = ?2",
        rusqlite::params![user, key],
        |r| r.get(0),
      )
    })
    .unwrap()
}

/// 三条错题：①字段全有；②可选字段全无、列表全空；③`pages` 存在但起止都为空（与 ② 的「没有 pages」不同）。
fn mistake_book() -> String {
  let question = |id: Option<&str>, full: bool| QuestionItem {
    id: id.map(str::to_owned),
    codes: Codes {
      j: full.then(|| "J1".to_owned()),
      p: full.then(|| "1.1".to_owned()),
    },
    question: "业余无线电台的呼号由谁分配？".into(),
    options: if full {
      vec![
        QuestionOption {
          key: "A".into(),
          text: "无线电管理机构".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "电台自己".into(),
        },
      ]
    } else {
      vec![]
    },
    answer_keys: if full { vec!["A".into()] } else { vec![] },
    kind: if full {
      QuestionType::Single
    } else {
      QuestionType::Multiple
    },
    pages: None,
    image_url: full.then(|| "/questions/images/0.jpg".to_owned()),
    explanation: full.then(|| "由管理机构统一分配。".to_owned()),
  };
  let record =
    |key: &str, question: QuestionItem, my: &[&str], banks: &[Bank], cause: Option<&str>| {
      MistakeRecord {
        key: key.into(),
        question,
        my_answer: my.iter().map(|s| (*s).to_owned()).collect(),
        wrong_count: 2,
        streak: 1,
        last_wrong_ms: 1_700_000_000_000,
        due_ms: 1_700_086_400_000,
        banks: banks.iter().copied().collect(),
        ease: 2.3,
        interval_days: 1.5,
        last_review_ms: 1_700_000_000_000,
        cause: cause.map(str::to_owned),
      }
    };
  let mut first = record(
    "0123456789abcdef",
    question(Some("A-1"), true),
    &["B", "C"],
    &[Bank::A, Bank::B],
    Some("memory"),
  );
  first.question.pages = Some(Pages {
    start: Some(3),
    end: Some(5),
  });
  let second = record("fedcba9876543210", question(None, false), &[], &[], None);
  let mut third = record(
    "00000000000000ff",
    question(Some("C-9"), false),
    &[],
    &[],
    None,
  );
  third.question.pages = Some(Pages {
    start: None,
    end: None,
  });
  serde_json::to_string(&MistakeBook {
    records: vec![first, second, third],
  })
  .unwrap()
}

#[test]
fn a_mistake_book_round_trips_through_its_child_tables_field_by_field() {
  let (store, user) = open();
  let raw = mistake_book();
  let m = map(&[("mistake-book", &raw)]);
  let back = round_trip(&store, user, &m);
  assert_eq!(back, m);
  // 回来的是同样的三条，字段逐个对得上（不只是文本相等）。
  let got: MistakeBook = serde_json::from_str(&back["mistake-book"]).unwrap();
  let want: MistakeBook = serde_json::from_str(&raw).unwrap();
  assert_eq!(got, want);
  assert_eq!(
    got.records[0].question.pages,
    Some(Pages {
      start: Some(3),
      end: Some(5)
    })
  );
  assert_eq!(
    got.records[1].question.pages, None,
    "没有 pages 与 pages 为空是两回事"
  );
  assert_eq!(
    got.records[2].question.pages,
    Some(Pages {
      start: None,
      end: None
    })
  );
  assert_eq!(got.records[0].my_answer, ["B", "C"]);
  assert_eq!(
    got.records[0].question.answer_keys,
    ["A"],
    "标准答案与我的作答不能串"
  );
  assert_eq!(got.records[1].question.id, None);
  assert_eq!(got.records[1].cause, None);

  // 题库是真正的多对多：能直接问「哪些错题在 B 类里错过」。
  let in_b: Vec<String> = store
    .with(|c| {
      c.prepare("SELECT key FROM mistake_banks WHERE user_id = ?1 AND bank = 'B'")?
        .query_map([user], |r| r.get(0))?
        .collect()
    })
    .unwrap();
  assert_eq!(in_b, ["0123456789abcdef"]);
}

#[test]
fn every_dedicated_table_takes_its_canonical_value_and_gives_it_back() {
  let (store, user) = open();
  let cases = cases();
  let all: Map = cases
    .iter()
    .map(|c| (c.key.to_owned(), c.raw.clone()))
    .collect();
  // 一次性全放进去，也验证各张表之间互不干扰。
  assert_eq!(round_trip(&store, user, &all), all);
  for case in &cases {
    assert_eq!(
      storage_of(&store, user, case.key),
      "table",
      "{} 的规范值应当进专表而不是原样存放",
      case.key
    );
    for (table, n) in case.tables {
      assert_eq!(rows(&store, table, user), *n, "{} → {table}", case.key);
    }
  }
}

#[test]
fn each_table_works_on_its_own_too() {
  // 单独放一个 key：排除「只有和别的 key 在一起才碰巧对」的可能。
  for case in cases() {
    let (store, user) = open();
    let m = map(&[(case.key, &case.raw)]);
    assert_eq!(round_trip(&store, user, &m), m, "{}", case.key);
    assert_eq!(storage_of(&store, user, case.key), "table", "{}", case.key);
  }
}

#[test]
fn a_family_of_keys_gets_one_row_each_and_changes_only_touch_their_own() {
  let (store, user) = open();
  let a = canon::<PracticeSavedState>(
    r#"{"version":2,"bank":"A","versionId":"2025-10","timestamp":1,"index":2,"order":"sequential","showAnswer":false,"orderIndices":[0,1],"answersByPosition":[["A"],null],"total":2}"#,
  );
  // 没有版本号的短写法 key 也是这一族。
  let b = canon::<PracticeSavedState>(
    r#"{"version":1,"bank":"B","timestamp":2,"index":0,"order":"random","showAnswer":true,"showExplanation":true,"orderIndices":[1,0],"answersByPosition":[null,null],"total":2}"#,
  );
  let mut m = map(&[("practice:2025-10:A", &a), ("practice:B", &b)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "practice_progress", user), 2);

  // 改其中一个：另一个的行不被碰（用 rowid 当记号）。
  let before: i64 = store
    .with(|c| {
      c.query_row(
        "SELECT rowid FROM practice_progress WHERE key = 'practice:B'",
        [],
        |r| r.get(0),
      )
    })
    .unwrap();
  m.insert(
    "practice:2025-10:A".into(),
    a.replace("\"timestamp\":1", "\"timestamp\":99"),
  );
  assert_eq!(round_trip(&store, user, &m), m);
  let after: i64 = store
    .with(|c| {
      c.query_row(
        "SELECT rowid FROM practice_progress WHERE key = 'practice:B'",
        [],
        |r| r.get(0),
      )
    })
    .unwrap();
  assert_eq!(before, after, "没改的那个 key 的行不该被删了重写");

  // 去掉一个：只有它那一行没了。
  m.remove("practice:B");
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "practice_progress", user), 1);
}

#[test]
fn valid_values_in_an_older_or_hand_written_shape_are_imported_and_normalized() {
  let (store, user) = open();
  let m = map(&[
    // 缺字段：补默认值。
    ("study-stats", r#"{"answered":3}"#),
    (
      "question-stats",
      r#"{"items":{"k":{"attempts":1,"correct":1,"last_ms":1}}}"#,
    ),
    // 键序不是字典序：有序映射按字典序输出。
    ("rpg-stars", r#"{"best":{"B":1,"A":2}}"#),
    // 影像：缺 padding、`image/jpg`、类型名大小写不同。
    ("qsl-image:1", "data:image/png;base64,iVBORw0KGgo"),
    ("qsl-image:2", "data:image/jpg;base64,/9j/4AAQ"),
    ("qsl-image:3", "data:IMAGE/WEBP;base64,UklGRg=="),
  ]);
  let back = round_trip(&store, user, &m);
  for key in m.keys() {
    assert_eq!(
      storage_of(&store, user, key),
      "table",
      "{key} 内容有效，应当进专表"
    );
  }
  assert_eq!(
    back["study-stats"],
    r#"{"categories":{},"subs":{},"answered":3,"correct":0,"banks":{}}"#
  );
  assert_eq!(
    back["question-stats"],
    r#"{"items":{"k":{"attempts":1,"correct":1,"last_ms":1,"duration_ms":0}}}"#
  );
  assert_eq!(back["rpg-stars"], r#"{"best":{"A":2,"B":1}}"#);
  assert_eq!(back["qsl-image:1"], "data:image/png;base64,iVBORw0KGgo=");
  assert_eq!(back["qsl-image:2"], "data:image/jpeg;base64,/9j/4AAQ");
  assert_eq!(back["qsl-image:3"], "data:image/webp;base64,UklGRg==");
  // 规范化是幂等的。
  assert_eq!(round_trip(&store, user, &back), back);
}

#[test]
fn values_that_cannot_be_parsed_or_do_not_fit_stay_verbatim() {
  let (store, user) = open();
  let m = map(&[
    // 进度里的题库与 key 对不上：不能信这个 key。
    (
      "practice:2025-10:A",
      r#"{"version":2,"bank":"B","versionId":"2025-10","timestamp":1,"index":0,"order":"sequential","showAnswer":false,"orderIndices":[],"answersByPosition":[],"total":0}"#,
    ),
    // 同一族里根本不是进度的 key。
    ("practice:lastMode", "random"),
    ("practice:noResumePrompt:2025-10:A", "1"),
    // 影像：编号带前导零（不是本站写出的 key）、类型不在白名单、数据不是 base64。
    ("qsl-image:007", "data:image/png;base64,iVBORw0KGgo="),
    ("qsl-image:8", "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4="),
    ("qsl-image:9", "data:image/png;base64,这不是base64"),
    // 笔记里同一个 id 出现两次：库里 (用户, id) 是唯一的，装不下。
    ("question-notes", r#"{"A-1":"x","A-1":"y"}"#),
    // 值不是该有的类型。
    ("bookmark-groups", r#"{"groups":[{"name":1,"ids":[]}]}"#),
  ]);
  assert_eq!(round_trip(&store, user, &m), m);
  for key in m.keys() {
    assert_eq!(storage_of(&store, user, key), "inline", "{key}");
  }
  for table in [
    "practice_progress",
    "qsl_images",
    "question_notes",
    "bookmark_groups",
  ] {
    assert_eq!(rows(&store, table, user), 0, "{table}");
  }
}

#[test]
fn nullable_fields_keep_none_and_empty_apart() {
  let (store, user) = open();
  let none = canon::<ExamSavedState>(
    r#"{"version":2,"bank":"A","timestamp":1,"endAtMs":9,"index":0,"answersByPosition":[],"flagsByPosition":[],"total":0}"#,
  );
  let empty = canon::<ExamSavedState>(
    r#"{"version":2,"bank":"B","timestamp":1,"endAtMs":9,"index":0,"answersByPosition":[],"flagsByPosition":[],"total":0,"questionIds":[],"questionsSnapshot":[]}"#,
  );
  assert!(!none.contains("questionIds"), "没有的字段原文里就没有");
  assert!(empty.contains("\"questionIds\":[]"));
  let m = map(&[("exam:savedState:A", &none), ("exam:savedState:B", &empty)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "exam_progress", user), 2);
  let nulls: i64 = store
    .with(|c| {
      c.query_row(
        "SELECT COUNT(*) FROM exam_progress
         WHERE question_ids IS NULL AND questions_snapshot IS NULL",
        [],
        |r| r.get(0),
      )
    })
    .unwrap();
  assert_eq!(nulls, 1, "只有第一份的两个可选数组是 NULL");
}

#[test]
fn a_qsl_image_is_stored_as_binary_not_as_text() {
  let (store, user) = open();
  let m = map(&[("qsl-image:7", "data:image/png;base64,iVBORw0KGgo=")]);
  round_trip(&store, user, &m);
  let (mime, data): (String, Vec<u8>) = store
    .with(|c| {
      c.query_row(
        "SELECT mime, data FROM qsl_images WHERE user_id = ?1 AND qso_id = 7",
        [user],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
    })
    .unwrap();
  assert_eq!(mime, "image/png");
  assert_eq!(data, b"\x89PNG\r\n\x1a\n");
}

#[test]
fn parent_and_child_tables_go_together_and_clearing_an_account_leaves_nothing() {
  let (store, user) = open();
  let all: Map = cases()
    .iter()
    .map(|c| (c.key.to_owned(), c.raw.clone()))
    .collect();
  round_trip(&store, user, &all);

  // 去掉带子表的 key：父子都清空，没有孤儿行。
  let mut fewer = all.clone();
  for key in [
    "card-review",
    "bookmark-groups",
    "morse-koch",
    "study-stats",
  ] {
    fewer.remove(key);
  }
  assert_eq!(round_trip(&store, user, &fewer), fewer);
  for table in [
    "card_schedule",
    "card_states",
    "card_new_today",
    "bookmark_groups",
    "bookmark_group_items",
    "koch_progress",
    "koch_chars",
    "study_scopes",
    "study_tallies",
    "study_seen",
  ] {
    assert_eq!(rows(&store, table, user), 0, "{table}");
  }

  // 清空账号：所有带 user_id 的表都空，包括以后新增的（按表结构查，不按名单查）。
  store.with(|c| clear_all(c, user)).unwrap();
  let leftover = store
    .with(|c| {
      let tables: Vec<String> = c
        .prepare(
          "SELECT name FROM sqlite_master
           WHERE type = 'table' AND name NOT IN ('users', 'sessions', 'backup_meta')
             AND name NOT LIKE 'sqlite_%'",
        )?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
      let mut left = Vec::new();
      for t in tables {
        let n: i64 = c.query_row(
          &format!("SELECT COUNT(*) FROM {t} WHERE user_id = ?1"),
          [user],
          |r| r.get(0),
        )?;
        if n != 0 {
          left.push((t, n));
        }
      }
      Ok(left)
    })
    .unwrap();
  assert!(leftover.is_empty(), "清空账号后还有残留：{leftover:?}");
}

#[test]
fn an_image_over_the_stored_limit_stays_inline_and_never_reaches_the_image_table() {
  use ham_web_core::qsl_image::MAX_STORED_CHARS;

  use super::Table;
  use super::images::QslImages;

  let prefix = "data:image/png;base64,";
  // 恰好到上限：收；多一个字符：不是本站写出的数据，`decode` 认不出，
  // `Entry::new` 会把认不出的值原样留在目录里（一个字节都不丢，但仍计入账号的总量上限）。
  let at_limit = format!("{prefix}{}", "A".repeat(MAX_STORED_CHARS - prefix.len()));
  assert!(QslImages::decode("qsl-image:7", &at_limit).is_some());
  let over = format!("{at_limit}A");
  assert!(QslImages::decode("qsl-image:7", &over).is_none());
  // 寻常大小的影像照旧进影像表。
  assert!(QslImages::decode("qsl-image:7", "data:image/png;base64,iVBORw0KGgo=").is_some());
}
