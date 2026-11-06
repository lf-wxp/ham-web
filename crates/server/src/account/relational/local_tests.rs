//! 页面自己的小文档、工具与器材、NEC 设计存档、错题暂存这几张表的测试。
//!
//! 夹具一律用类型自己的序列化生成（见 `tables_tests` 的说明）。除了「规范值进表、原样取回」，
//! 还专门核对几件容易悄悄出错的事：枚举列的 `CHECK` 覆盖每一个变体、`f32` 放进 `REAL` 不走样、
//! 一族 key 互不干扰、`NULL` 与空串分得开。

use ham_web_core::checkin::CheckinState;
use ham_web_core::contest::ContestSession;
use ham_web_core::countdown::{CountdownEvent, CountdownList};
use ham_web_core::diy_projects::DiyProgressBook;
use ham_web_core::gear_user::UserMeasurements;
use ham_web_core::grid_filter::{GridFilters, QslFilter};
use ham_web_core::listen::{Settings as ListenSettings, Source};
use ham_web_core::logbook::StationInfo;
use ham_web_core::morse_store::{MorseSettings, MorseStats, SendStats, TrainerSettings};
use ham_web_core::morse_trainer::{MODES, Scope, Target};
use ham_web_core::nec_designs::{
  DesignBook, SavedDesign, SavedFeed, SavedGround, SavedLine, SavedLoad, SavedModel, SavedWire,
};
use ham_web_core::noise_floor::Sample;
use ham_web_core::question::{Codes, Pages, QuestionItem, QuestionOption, QuestionType};

use super::tables_tests::{Case, canon, rows, storage_of};
use super::tests::{map, open, round_trip};

fn json<T: serde::Serialize>(v: &T) -> String {
  serde_json::to_string(v).unwrap()
}

fn design(id: u64, name: &str, ground: SavedGround) -> SavedDesign {
  SavedDesign {
    id,
    name: name.into(),
    saved_at: "2026-11-05".into(),
    model: SavedModel {
      wires: vec![
        SavedWire {
          a: [0.0, 0.0, 5.0],
          b: [0.0, 0.0, 15.0],
          radius: 0.001,
          segments: 21,
        },
        SavedWire {
          a: [0.0, 0.0, 5.0],
          b: [4.5, 0.0, 5.0],
          radius: 0.0008,
          segments: 11,
        },
      ],
      feeds: vec![SavedFeed {
        wire: 0,
        at: 0.5,
        volts: 1.0,
      }],
      loads: vec![SavedLoad {
        wire: 1,
        at: 0.9,
        r_ohm: 50.0,
        l_uh: 0.5,
        c_pf: 12.5,
      }],
      freq_hz: 14.2e6,
      ground,
      conductivity: Some(5.8e7),
      lines: vec![SavedLine {
        wire_a: 0,
        at_a: 0.1,
        wire_b: 1,
        at_b: 0.2,
        z0: 75.0,
        length_m: 3.2,
        velocity_factor: 0.66,
      }],
    },
  }
}

fn question(id: &str, full: bool) -> QuestionItem {
  QuestionItem {
    id: Some(id.to_owned()),
    codes: Codes {
      j: full.then(|| "J7".to_owned()),
      p: full.then(|| "2.1".to_owned()),
    },
    question: "天线的增益单位是什么？".into(),
    options: if full {
      vec![
        QuestionOption {
          key: "A".into(),
          text: "dBi".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "dBm".into(),
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
    pages: full.then_some(Pages {
      start: Some(3),
      end: None,
    }),
    image_url: full.then(|| "/questions/images/3.jpg".to_owned()),
    explanation: full.then(|| "增益相对各向同性辐射器。".to_owned()),
  }
}

pub(super) fn cases() -> Vec<Case> {
  vec![
    Case {
      key: "countdowns",
      raw: json(&CountdownList {
        events: vec![
          CountdownEvent {
            id: 1,
            title: "A 类考试".into(),
            target_ms: 1_800_000_000_000,
          },
          CountdownEvent {
            id: 3,
            title: "执照到期 \"重要\"".into(),
            target_ms: 1_900_000_000_000,
          },
        ],
      }),
      tables: &[("countdown_events", 2)],
    },
    Case {
      key: "daily-checkin",
      raw: json(&CheckinState {
        last_date: "20261105".into(),
        streak: 12,
      }),
      tables: &[("daily_checkin", 1)],
    },
    Case {
      key: "contest-session",
      raw: json(&{
        let mut s = ContestSession::starting("2026-11-05 08:00".into());
        s.station_id = 2;
        s.my_exch = "24".into();
        s.target_qsos = 300;
        s.planned_minutes = 600;
        s
      }),
      tables: &[("contest_session", 1)],
    },
    Case {
      key: "grid-map-filters",
      raw: json(&GridFilters {
        band: Some("20m".into()),
        mode: None,
        dxcc: Some("318".into()),
        year: None,
        callsign: "BG".into(),
        qsl: Some(QslFilter::SentPending),
        paths: true,
      }),
      tables: &[("grid_filters", 1)],
    },
    Case {
      key: "listen-settings",
      raw: json(&{
        let mut s = ListenSettings {
          source: Source::Mistakes,
          rate: 1.2,
          think_secs: 8,
          explain: true,
          shuffle: false,
          positions: Default::default(),
        };
        s.positions.insert(Source::A, 17);
        s.positions.insert(Source::Bookmarks, 4);
        s
      }),
      tables: &[("listen_settings", 1), ("listen_positions", 2)],
    },
    Case {
      key: "morse-stats",
      raw: json(&MorseStats {
        correct: 40,
        wrong: 9,
        mistakes: [
          ("K".to_owned(), 4),
          ("Q".to_owned(), 2),
          ("73".to_owned(), 1),
        ]
        .into(),
        streak: 0,
        best_streak: 11,
        top_wpm: 22.5,
        today: "2026-11-05".into(),
        today_correct: 6,
        today_wrong: 1,
        daily: [
          ("2026-11-03".to_owned(), (8, 2)),
          ("2026-11-04".to_owned(), (5, 0)),
        ]
        .into(),
      }),
      tables: &[
        ("morse_stats", 1),
        ("morse_mistakes", 3),
        ("morse_daily", 2),
      ],
    },
    Case {
      key: "morse-trainer",
      raw: json(&TrainerSettings {
        mode: MODES[2],
        scope: Scope::Weak,
        target: Target::CallSign,
        wpm: 18.0,
        eff_wpm: 12.5,
        adaptive: true,
      }),
      tables: &[("morse_trainer", 1)],
    },
    Case {
      key: "morse-send",
      raw: json(&SendStats {
        correct: 30,
        wrong: 4,
        best_streak: 9,
      }),
      tables: &[("morse_send", 1)],
    },
    Case {
      key: "morse-settings",
      raw: json(&MorseSettings {
        tone_hz: 650,
        volume: 0.3,
      }),
      tables: &[("morse_settings", 1)],
    },
    Case {
      key: "station-info",
      raw: json(&StationInfo {
        callsign: "BG4XXX".into(),
        operator: "张三".into(),
        gridsquare: "OM89".into(),
        rig: "IC-7300".into(),
        antenna: "GP".into(),
      }),
      tables: &[("station_info", 1)],
    },
    Case {
      key: "gear-measurements",
      raw: canon::<UserMeasurements>(
        r#"{"ic-7300":{"current_a":1.4,"noise_floor_dbm":-126.5,"rmdr_db":null,"phase_noise_10k_dbc":-135.0,"measured_on":"2026-10-08"},"ft-710":{"current_a":null,"noise_floor_dbm":null,"rmdr_db":98.0,"phase_noise_10k_dbc":null,"measured_on":""}}"#,
      ),
      tables: &[("gear_measurements", 2)],
    },
    Case {
      key: "diy-progress",
      raw: canon::<DiyProgressBook>(
        r#"{"天线分析仪":{"done":true,"bought":["SMA 座","外壳"]},"空心线圈":{"done":false,"bought":[]}}"#,
      ),
      tables: &[("diy_progress", 2), ("diy_bought", 2)],
    },
    Case {
      key: "noise-floor-samples",
      raw: json(&vec![
        Sample {
          label: "40m 夜间".into(),
          at_ms: 1_762_300_000_000.0,
          floor_db: -97.3,
          occupied_ratio: 0.12,
        },
        Sample {
          label: "40m 白天".into(),
          at_ms: 1_762_390_000_000.0,
          floor_db: -101.7,
          occupied_ratio: 0.31,
        },
      ]),
      tables: &[("noise_samples", 2)],
    },
    Case {
      key: "nec-designs",
      raw: json(&DesignBook {
        designs: vec![
          design(
            2,
            "八木",
            SavedGround::Lossy {
              eps_r: 13.0,
              sigma: 0.005,
            },
          ),
          design(1, "垂直振子", SavedGround::Perfect),
        ],
      }),
      tables: &[
        ("nec_designs", 2),
        ("nec_wires", 4),
        ("nec_feeds", 2),
        ("nec_loads", 2),
        ("nec_lines", 2),
      ],
    },
    Case {
      key: "exam:wrong:A",
      raw: json(&vec![question("A-1", true), question("A-2", false)]),
      tables: &[
        ("wrong_questions", 2),
        ("wrong_question_options", 2),
        ("wrong_question_answers", 1),
      ],
    },
  ]
}

#[test]
fn every_enum_column_accepts_every_variant_of_its_enum() {
  // 表里的 `CHECK` 是手写的取值清单；枚举加了变体而没改清单，落库时才会失败（整次导入返回 500）。
  // 这里逐个变体走一遍，加变体时这条测试会第一个红。
  let (store, user) = open();
  let mut all = map(&[]);
  for mode in MODES {
    for scope in Scope::ALL {
      for target in Target::ALL {
        let s = TrainerSettings {
          mode,
          scope,
          target,
          ..TrainerSettings::default()
        };
        let m = map(&[("morse-trainer", &json(&s))]);
        assert_eq!(
          round_trip(&store, user, &m),
          m,
          "{mode:?} {scope:?} {target:?}"
        );
        assert_eq!(storage_of(&store, user, "morse-trainer"), "table");
      }
    }
  }
  for qsl in QslFilter::ALL {
    let f = GridFilters {
      qsl: Some(qsl),
      ..GridFilters::default()
    };
    let m = map(&[("grid-map-filters", &json(&f))]);
    assert_eq!(round_trip(&store, user, &m), m, "{qsl:?}");
    assert_eq!(storage_of(&store, user, "grid-map-filters"), "table");
  }
  for source in Source::ALL {
    let mut s = ListenSettings {
      source,
      ..ListenSettings::default()
    };
    for other in Source::ALL {
      s.positions.insert(other, 1);
    }
    let m = map(&[("listen-settings", &json(&s))]);
    assert_eq!(round_trip(&store, user, &m), m, "{source:?}");
    assert_eq!(storage_of(&store, user, "listen-settings"), "table");
  }
  all.clear();
}

#[test]
fn f32_values_survive_the_real_column_exactly() {
  let (store, user) = open();
  // 0.1 / 0.3 / 0.7 这类数转成 f64 再转回 f32 必须还是同一个 f32，序列化出来的文本才不变。
  for v in [0.1_f32, 0.3, 0.7, 0.8, 1.2, 1.5, 0.05, 1.0, 0.0] {
    let settings = MorseSettings {
      tone_hz: 700,
      volume: v,
    };
    let listen = ListenSettings {
      rate: v,
      ..ListenSettings::default()
    };
    let m = map(&[
      ("morse-settings", &json(&settings)),
      ("listen-settings", &json(&listen)),
    ]);
    assert_eq!(round_trip(&store, user, &m), m, "{v}");
  }
  let samples = vec![
    Sample {
      label: "x".into(),
      at_ms: 1.0,
      floor_db: -97.3,
      occupied_ratio: 0.123_456_79,
    },
    Sample {
      label: "y".into(),
      at_ms: 2.5,
      floor_db: -200.0,
      occupied_ratio: 1.0,
    },
  ];
  let m = map(&[("noise-floor-samples", &json(&samples))]);
  assert_eq!(round_trip(&store, user, &m), m);
}

#[test]
fn out_of_range_f32_values_stay_verbatim_instead_of_failing_the_import() {
  let (store, user) = open();
  // 手改成超出 f32 范围的数：`serde_json` 解析就报错（不会得到无穷），所以这个 key 进不了表；
  // 它原样留在目录里，既不丢，也不拦住同一份备份里别的 key。
  let m = map(&[
    ("morse-settings", r#"{"tone_hz":700,"volume":1e39}"#),
    ("listen-settings", r#"{"rate":1e39}"#),
    (
      "noise-floor-samples",
      r#"[{"label":"x","at_ms":1.0,"floor_db":-1e39,"occupied_ratio":0.1}]"#,
    ),
    ("daily-checkin", r#"{"last_date":"20261105","streak":1}"#),
  ]);
  let back = round_trip(&store, user, &m);
  for key in ["morse-settings", "listen-settings", "noise-floor-samples"] {
    assert_eq!(back[key], m[key], "{key} 要原样留着");
    assert_eq!(storage_of(&store, user, key), "inline", "{key}");
  }
  assert_eq!(
    storage_of(&store, user, "daily-checkin"),
    "table",
    "别的 key 不受影响"
  );
}

#[test]
fn the_morse_run_streak_is_runtime_only_and_never_stored() {
  let (store, user) = open();
  // `streak` 带 `#[serde(skip)]`：JSON 里没有它，库里也没有，读回来是 0。
  let stats = MorseStats {
    streak: 7,
    best_streak: 9,
    ..MorseStats::default()
  };
  let raw = json(&stats);
  assert!(!raw.contains("\"streak\""));
  let back = round_trip(&store, user, &map(&[("morse-stats", &raw)]));
  let got: MorseStats = serde_json::from_str(&back["morse-stats"]).unwrap();
  assert_eq!((got.streak, got.best_streak), (0, 9));
}

#[test]
fn older_or_hand_written_documents_are_normalized_on_import() {
  let (store, user) = open();
  let m = map(&[
    // 缺字段补默认值：竞赛场次的可选字段、听题设置整体。
    (
      "contest-session",
      r#"{"contest_id":"x","start":"2026-01-01 00:00","my_exch":"","freq":"7","mode":"SSB"}"#,
    ),
    ("listen-settings", r#"{"source":"b"}"#),
    ("morse-trainer", r#"{"wpm":15.0}"#),
    // 键序不是字典序：有序映射输出字典序。
    (
      "diy-progress",
      r#"{"b":{"done":true},"a":{"bought":["z","y"]}}"#,
    ),
  ]);
  let back = round_trip(&store, user, &m);
  for key in m.keys() {
    assert_eq!(
      storage_of(&store, user, key),
      "table",
      "{key} 内容有效，应当进专表"
    );
  }
  let session: ContestSession = serde_json::from_str(&back["contest-session"]).unwrap();
  assert_eq!((session.power.as_str(), session.target_qsos), ("", 0));
  let listen: ListenSettings = serde_json::from_str(&back["listen-settings"]).unwrap();
  assert_eq!(
    (listen.source, listen.think_secs, listen.rate),
    (Source::B, 5, 1.0)
  );
  let trainer: TrainerSettings = serde_json::from_str(&back["morse-trainer"]).unwrap();
  assert_eq!((trainer.wpm, trainer.eff_wpm), (15.0, 20.0));
  assert_eq!(
    back["diy-progress"],
    r#"{"a":{"done":false,"bought":["y","z"]},"b":{"done":true,"bought":[]}}"#
  );
  // 规范化是幂等的。
  assert_eq!(round_trip(&store, user, &back), back);
}

#[test]
fn duplicate_natural_keys_in_the_new_tables_fall_back_to_verbatim() {
  let (store, user) = open();
  let dup_events = json(&CountdownList {
    events: vec![
      CountdownEvent {
        id: 1,
        title: "a".into(),
        target_ms: 1,
      },
      CountdownEvent {
        id: 1,
        title: "b".into(),
        target_ms: 2,
      },
    ],
  });
  let dup_designs = json(&DesignBook {
    designs: vec![
      design(5, "x", SavedGround::Free),
      design(5, "y", SavedGround::Free),
    ],
  });
  let m = map(&[("countdowns", &dup_events), ("nec-designs", &dup_designs)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "countdown_events", user), 0);
  assert_eq!(rows(&store, "nec_designs", user), 0);
  assert_eq!(storage_of(&store, user, "countdowns"), "inline");
}

#[test]
fn nec_designs_keep_their_order_ids_and_wire_indices() {
  let (store, user) = open();
  // 最新的在前：id 不是升序；馈电 / 负载 / 传输线用导线的下标引用导线，下标必须原样回来。
  let book = DesignBook {
    designs: vec![
      design(
        9,
        "后存的",
        SavedGround::Lossy {
          eps_r: 5.0,
          sigma: 0.001,
        },
      ),
      design(4, "先存的", SavedGround::Free),
    ],
  };
  let raw = json(&book);
  let back = round_trip(&store, user, &map(&[("nec-designs", &raw)]));
  let got: DesignBook = serde_json::from_str(&back["nec-designs"]).unwrap();
  assert_eq!(got, book);
  assert_eq!(got.designs[0].id, 9);
  assert_eq!(got.designs[0].model.loads[0].wire, 1);
  assert_eq!(got.designs[1].model.ground, SavedGround::Free);
  // 大地参数只在 lossy 时有值，其余是 NULL（表里的 CHECK 守着）。
  let lossy: i64 = store
    .with(|c| {
      c.query_row(
        "SELECT COUNT(*) FROM nec_designs WHERE ground = 'lossy' AND eps_r = 5.0 AND sigma = 0.001",
        [],
        |r| r.get(0),
      )
    })
    .unwrap();
  assert_eq!(lossy, 1);
  // 一个设计没有负载 / 传输线（它们是 `#[serde(default)]` 的可选项）也行。
  let mut bare = design(1, "最简", SavedGround::Perfect);
  bare.model.loads.clear();
  bare.model.lines.clear();
  bare.model.conductivity = None;
  let raw = json(&DesignBook {
    designs: vec![bare.clone()],
  });
  let back = round_trip(&store, user, &map(&[("nec-designs", &raw)]));
  let got: DesignBook = serde_json::from_str(&back["nec-designs"]).unwrap();
  assert_eq!(got.designs, vec![bare]);
  assert_eq!(rows(&store, "nec_loads", user), 0);
}

#[test]
fn wrong_question_banks_are_independent_rows_of_one_family() {
  let (store, user) = open();
  let a = json(&vec![question("A-1", true), question("A-2", false)]);
  let b = json(&vec![question("B-9", true)]);
  let mut m = map(&[("exam:wrong:A", &a), ("exam:wrong:B", &b)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "wrong_questions", user), 3);
  assert_eq!(rows(&store, "wrong_question_options", user), 4);

  // 清掉一个题库：只有它的行没了，另一个的行与子行都在。
  m.remove("exam:wrong:A");
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "wrong_questions", user), 1);
  assert_eq!(rows(&store, "wrong_question_options", user), 2);
  assert_eq!(rows(&store, "wrong_question_answers", user), 1);

  // 题库后缀不是规范写法（小写、不存在的题库）：不能认，否则两个 key 会互相覆盖同一批行。
  let odd = map(&[
    ("exam:wrong:a", &a),
    ("exam:wrong:D", &a),
    ("exam:wrong:A:extra", &a),
    ("exam:wrong:", &a),
  ]);
  assert_eq!(round_trip(&store, user, &odd), odd);
  for key in odd.keys() {
    assert_eq!(storage_of(&store, user, key), "inline", "{key}");
  }
  assert_eq!(rows(&store, "wrong_questions", user), 0);
}

#[test]
fn wrong_questions_keep_no_pages_and_empty_pages_apart() {
  let (store, user) = open();
  let mut with_empty = question("A-1", false);
  with_empty.pages = Some(Pages {
    start: None,
    end: None,
  });
  let without = question("A-2", false);
  let raw = json(&vec![with_empty.clone(), without.clone()]);
  let back = round_trip(&store, user, &map(&[("exam:wrong:C", &raw)]));
  let got: Vec<QuestionItem> = serde_json::from_str(&back["exam:wrong:C"]).unwrap();
  assert_eq!(got, vec![with_empty, without]);
}

#[test]
fn a_blank_gear_entry_and_an_empty_diy_project_are_kept_as_they_are() {
  let (store, user) = open();
  // 应用自己会把全空的记录删掉，但导入的文件里可能有：照收，不替用户做主。
  let gear = canon::<UserMeasurements>(
    r#"{"empty-rig":{"current_a":null,"noise_floor_dbm":null,"rmdr_db":null,"phase_noise_10k_dbc":null,"measured_on":""}}"#,
  );
  let diy = canon::<DiyProgressBook>(r#"{"project":{"done":false,"bought":[]}}"#);
  let m = map(&[("gear-measurements", &gear), ("diy-progress", &diy)]);
  assert_eq!(round_trip(&store, user, &m), m);
  assert_eq!(rows(&store, "gear_measurements", user), 1);
  assert_eq!(rows(&store, "diy_progress", user), 1);
  assert_eq!(rows(&store, "diy_bought", user), 0);
}

#[test]
fn negative_zero_is_normalized_once_and_then_stable() {
  // SQLite 会把 -0.0 存成 0（整数值的 REAL 按整数存）。内容数值上没变，但文本从 `-0.0` 变成 `0.0`：
  // 只允许变这一次，再导入一遍不能再变（否则同步基线会一直漂）。
  let (store, user) = open();
  let mut d = design(1, "x", SavedGround::Free);
  d.model.wires[0].a = [-0.0, 0.0, 5.0];
  let raw = json(&DesignBook { designs: vec![d] });
  let once = round_trip(&store, user, &map(&[("nec-designs", &raw)]));
  let twice = round_trip(&store, user, &once);
  assert_eq!(once, twice);
}
