//! 账号接口的端到端测试：走完整的路由与中间件，数据库用内存库。

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use ham_web_core::account::{BackupInfo, SessionInfo};
use ham_web_core::delta::{manifest_fingerprint, manifest_of};
use ham_web_core::remote_sync::Fingerprint;
use tower::ServiceExt;

use super::{AccountConfig, AccountService, CookieSecure};

struct Reply {
  status: StatusCode,
  headers: HeaderMap,
  body: String,
}

impl Reply {
  fn json(&self) -> serde_json::Value {
    serde_json::from_str(&self.body).unwrap_or(serde_json::Value::Null)
  }

  fn code(&self) -> String {
    self.json()["error"]["code"]
      .as_str()
      .unwrap_or_default()
      .to_owned()
  }

  /// `Set-Cookie` 里的 `ham_session=…`（可能是空值 = 清除）。
  fn cookie(&self) -> Option<String> {
    self
      .headers
      .get("set-cookie")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.split(';').next())
      .map(str::to_owned)
  }
}

async fn call(
  app: &Router,
  method: Method,
  uri: &str,
  cookie: Option<&str>,
  extra: &[(&str, &str)],
  body: Option<&str>,
) -> Reply {
  let mut req = Request::builder().method(method).uri(uri);
  if let Some(c) = cookie {
    req = req.header("cookie", c);
  }
  for (k, v) in extra {
    req = req.header(*k, *v);
  }
  let res = app
    .clone()
    .oneshot(
      req
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_owned())))
        .unwrap(),
    )
    .await
    .unwrap();
  let (parts, body) = res.into_parts();
  let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
  Reply {
    status: parts.status,
    headers: parts.headers,
    body: String::from_utf8_lossy(&bytes).into_owned(),
  }
}

const JSON: (&str, &str) = ("content-type", "application/json");

async fn post(app: &Router, uri: &str, cookie: Option<&str>, body: &str) -> Reply {
  call(app, Method::POST, uri, cookie, &[JSON], Some(body)).await
}

fn creds(user: &str, pass: &str) -> String {
  serde_json::json!({ "username": user, "password": pass }).to_string()
}

async fn register(app: &Router, user: &str, pass: &str) -> Reply {
  post(app, "/api/account/register", None, &creds(user, pass)).await
}

/// 注册并返回会话 Cookie。
async fn signed_up(app: &Router, user: &str) -> String {
  let r = register(app, user, "correct horse").await;
  assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
  r.cookie().expect("注册应下发会话")
}

fn app() -> Router {
  AccountService::in_memory(true).router()
}

const BACKUP: &str = r#"{"bookmarks":"[1,2]","logbook":"{}"}"#;

/// 前端对同一份备份算的指纹：对 `(key, 摘要)` 的清单求，设备专属的 key 不算。
fn backup_fp(json: &str) -> Fingerprint {
  let map: std::collections::BTreeMap<String, String> = serde_json::from_str(json).unwrap();
  manifest_fingerprint(&manifest_of(&map, true))
}

async fn put(app: &Router, cookie: &str, extra: &[(&str, &str)], body: &str) -> Reply {
  let mut headers = vec![JSON];
  headers.extend_from_slice(extra);
  call(
    app,
    Method::PUT,
    "/api/account/backup",
    Some(cookie),
    &headers,
    Some(body),
  )
  .await
}

async fn get_backup(app: &Router, cookie: &str) -> Reply {
  call(
    app,
    Method::GET,
    "/api/account/backup",
    Some(cookie),
    &[],
    None,
  )
  .await
}

#[tokio::test]
async fn without_a_database_every_endpoint_says_unavailable() {
  let app = AccountService::disabled().router();
  let s = call(&app, Method::GET, "/api/account/session", None, &[], None).await;
  assert_eq!(s.status, StatusCode::OK);
  let info: SessionInfo = serde_json::from_str(&s.body).unwrap();
  assert!(!info.enabled && info.user.is_none());
  let r = register(&app, "alice", "correct horse").await;
  assert_eq!(
    (r.status, r.code().as_str()),
    (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
  );
  let b = get_backup(&app, "ham_session=x").await;
  assert_eq!(b.code(), "unavailable");
  // 退出登录始终是幂等的，哪怕服务没启用。
  let l = post(&app, "/api/account/logout", None, "").await;
  assert_eq!(l.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn registering_signs_you_in_and_the_session_reports_who_you_are() {
  let app = app();
  let anon = call(&app, Method::GET, "/api/account/session", None, &[], None).await;
  let info: SessionInfo = serde_json::from_str(&anon.body).unwrap();
  // 未登录是 200 + `user: null`，不是 401。
  assert_eq!(anon.status, StatusCode::OK);
  assert!(info.enabled && info.registration_open && info.user.is_none());

  let r = register(&app, "BG4XXX", "correct horse").await;
  assert_eq!(r.status, StatusCode::CREATED);
  let cookie = r.cookie().unwrap();
  assert!(cookie.starts_with("ham_session=") && cookie.len() > 20);
  let set = r.headers["set-cookie"].to_str().unwrap();
  assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict"));

  let s = call(
    &app,
    Method::GET,
    "/api/account/session",
    Some(&cookie),
    &[],
    None,
  )
  .await;
  let user = serde_json::from_str::<SessionInfo>(&s.body)
    .unwrap()
    .user
    .unwrap();
  assert_eq!(user.username, "BG4XXX");
  assert!(user.backup.is_none());
  // 响应里没有口令哈希之类的东西。
  assert!(!s.body.contains("argon2") && !s.body.contains("password"));
}

#[tokio::test]
async fn usernames_are_unique_ignoring_case_and_credentials_are_validated() {
  let app = app();
  signed_up(&app, "BG4XXX").await;
  let dup = register(&app, "bg4xxx", "correct horse").await;
  assert_eq!(
    (dup.status, dup.code().as_str()),
    (StatusCode::CONFLICT, "username_taken")
  );
  let bad_name = register(&app, "a b", "correct horse").await;
  assert_eq!(
    (bad_name.status, bad_name.code().as_str()),
    (StatusCode::BAD_REQUEST, "invalid_username")
  );
  let short = register(&app, "carol", "short").await;
  assert_eq!(
    (short.status, short.code().as_str()),
    (StatusCode::BAD_REQUEST, "invalid_password")
  );
  let same = register(&app, "carol123", "CAROL123").await;
  assert_eq!(same.code(), "invalid_password");
}

#[tokio::test]
async fn login_gives_the_same_answer_for_a_wrong_password_and_an_unknown_user() {
  let app = app();
  signed_up(&app, "alice").await;
  let wrong = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "wrong password"),
  )
  .await;
  let ghost = post(
    &app,
    "/api/account/login",
    None,
    &creds("nobody", "wrong password"),
  )
  .await;
  assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
  assert_eq!(wrong.body, ghost.body);
  assert_eq!(wrong.code(), "bad_credentials");
  assert!(wrong.cookie().is_none());

  // 账号名不分大小写，口令分。
  let ok = post(
    &app,
    "/api/account/login",
    None,
    &creds("ALICE", "correct horse"),
  )
  .await;
  assert_eq!(ok.status, StatusCode::OK);
  assert!(ok.cookie().is_some());
  let upper = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "Correct Horse"),
  )
  .await;
  assert_eq!(upper.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn closed_registration_still_lets_existing_accounts_in() {
  let svc = AccountService::in_memory(false);
  let inner = svc.inner.clone().unwrap();
  let phc = super::password::hash("correct horse").unwrap();
  inner.store.create_user("alice", &phc, 0).unwrap();
  let app = svc.router();

  let r = register(&app, "bob", "correct horse").await;
  assert_eq!(
    (r.status, r.code().as_str()),
    (StatusCode::FORBIDDEN, "registration_closed")
  );
  let l = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "correct horse"),
  )
  .await;
  assert_eq!(l.status, StatusCode::OK);
  assert!(
    !serde_json::from_str::<SessionInfo>(&l.body)
      .unwrap()
      .registration_open
  );
}

#[tokio::test]
async fn json_endpoints_refuse_other_content_types() {
  let app = app();
  // 跨站表单只能发 urlencoded / multipart / text：拒绝它们是 SameSite 之外的第二道防线。
  for ct in ["text/plain", "application/x-www-form-urlencoded"] {
    let r = call(
      &app,
      Method::POST,
      "/api/account/register",
      None,
      &[("content-type", ct)],
      Some(&creds("alice", "correct horse")),
    )
    .await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::BAD_REQUEST, "bad_request"),
      "{ct}"
    );
  }
  let garbage = post(&app, "/api/account/login", None, "not json").await;
  assert_eq!(garbage.code(), "bad_request");
}

#[tokio::test]
async fn too_many_wrong_guesses_for_one_account_are_throttled() {
  let app = app();
  signed_up(&app, "alice").await;
  let mut last = None;
  for _ in 0..(super::USER_ATTEMPTS + 2) {
    last = Some(
      post(
        &app,
        "/api/account/login",
        None,
        &creds("alice", "wrong password"),
      )
      .await,
    );
  }
  let last = last.unwrap();
  assert_eq!(
    (last.status, last.code().as_str()),
    (StatusCode::TOO_MANY_REQUESTS, "too_many_attempts")
  );
  // 限流按账号名计，不分大小写：换个写法绕不过。
  let again = post(
    &app,
    "/api/account/login",
    None,
    &creds("ALICE", "correct horse"),
  )
  .await;
  assert_eq!(again.status, StatusCode::TOO_MANY_REQUESTS);
  // 别的账号不受牵连。
  let other = register(&app, "bob", "correct horse").await;
  assert_eq!(other.status, StatusCode::CREATED);
}

#[tokio::test]
async fn logging_out_kills_the_session_server_side() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  let out = post(&app, "/api/account/logout", Some(&cookie), "").await;
  assert_eq!(out.status, StatusCode::NO_CONTENT);
  assert_eq!(out.cookie().as_deref(), Some("ham_session="));
  assert!(
    out.headers["set-cookie"]
      .to_str()
      .unwrap()
      .contains("Max-Age=0")
  );
  // 就算有人留着旧 Cookie 重放，也已经无效。
  assert_eq!(get_backup(&app, &cookie).await.code(), "unauthorized");
  let s = call(
    &app,
    Method::GET,
    "/api/account/session",
    Some(&cookie),
    &[],
    None,
  )
  .await;
  assert!(
    serde_json::from_str::<SessionInfo>(&s.body)
      .unwrap()
      .user
      .is_none()
  );
}

#[tokio::test]
async fn the_cookie_is_marked_secure_behind_an_https_proxy() {
  let app = app();
  let plain = register(&app, "alice", "correct horse").await;
  assert!(
    !plain.headers["set-cookie"]
      .to_str()
      .unwrap()
      .contains("Secure")
  );
  let proxied = call(
    &app,
    Method::POST,
    "/api/account/register",
    None,
    &[JSON, ("x-forwarded-proto", "https")],
    Some(&creds("bob", "correct horse")),
  )
  .await;
  assert!(
    proxied.headers["set-cookie"]
      .to_str()
      .unwrap()
      .contains("Secure")
  );
}

#[tokio::test]
async fn cookie_security_can_be_forced_either_way() {
  for (mode, expect) in [(CookieSecure::Always, true), (CookieSecure::Never, false)] {
    let svc = AccountService::in_memory_with(AccountConfig {
      cookie_secure: mode,
      ..AccountConfig::default()
    });
    let r = call(
      &svc.router(),
      Method::POST,
      "/api/account/register",
      None,
      &[
        JSON,
        ("x-forwarded-proto", if expect { "http" } else { "https" }),
      ],
      Some(&creds("alice", "correct horse")),
    )
    .await;
    assert_eq!(
      r.headers["set-cookie"].to_str().unwrap().contains("Secure"),
      expect
    );
  }
}

#[tokio::test]
async fn backup_endpoints_need_a_session() {
  let app = app();
  for (method, extra) in [
    (Method::GET, vec![]),
    (Method::PUT, vec![JSON]),
    (Method::DELETE, vec![]),
  ] {
    let r = call(
      &app,
      method,
      "/api/account/backup",
      None,
      &extra,
      Some(BACKUP),
    )
    .await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::UNAUTHORIZED, "unauthorized")
    );
  }
  let forged = get_backup(
    &app,
    "ham_session=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
  )
  .await;
  assert_eq!(forged.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_backup_round_trips_byte_for_byte_with_the_clients_fingerprint() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  assert_eq!(
    get_backup(&app, &cookie).await.status,
    StatusCode::NO_CONTENT
  );

  let saved = put(&app, &cookie, &[], BACKUP).await;
  assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
  let info: BackupInfo = serde_json::from_str(&saved.body).unwrap();
  // 服务端算的指纹必须与前端对同一份 map 算的相同：基线与 `If-Match` 都靠它。
  assert_eq!(info.fingerprint, backup_fp(BACKUP));
  // 体积是各 key 规范文本的字节数之和（不含 JSON 外壳）。
  let values: std::collections::BTreeMap<String, String> = serde_json::from_str(BACKUP).unwrap();
  assert_eq!(
    info.size as usize,
    values.values().map(String::len).sum::<usize>()
  );

  let got = get_backup(&app, &cookie).await;
  assert_eq!(got.status, StatusCode::OK);
  assert_eq!(got.body, BACKUP);
  assert_eq!(
    got.headers["etag"].to_str().unwrap(),
    format!("\"{}\"", info.fingerprint.get())
  );

  // 会话探测里能看到云端概况。
  let s = call(
    &app,
    Method::GET,
    "/api/account/session",
    Some(&cookie),
    &[],
    None,
  )
  .await;
  let seen = serde_json::from_str::<SessionInfo>(&s.body)
    .unwrap()
    .user
    .unwrap()
    .backup;
  assert_eq!(seen, Some(info));

  let del = call(
    &app,
    Method::DELETE,
    "/api/account/backup",
    Some(&cookie),
    &[],
    None,
  )
  .await;
  assert_eq!(del.status, StatusCode::NO_CONTENT);
  assert_eq!(
    get_backup(&app, &cookie).await.status,
    StatusCode::NO_CONTENT
  );
}

#[tokio::test]
async fn a_canonical_backup_is_stored_in_tables_and_comes_back_whole() {
  use ham_web_core::exam_history::ExamRecord;
  use ham_web_core::logbook::{LogEntry, Logbook};

  let svc = AccountService::in_memory(true);
  let app = svc.clone().router();
  let cookie = signed_up(&app, "alice").await;

  // 前端写出来的就是各个核心类型的序列化：值都是这种「规范字节」，才会进专表。
  let logbook = serde_json::to_string(&Logbook {
    entries: (1..=3)
      .map(|id| LogEntry {
        id,
        date: "2026-01-02".into(),
        time: "03:04".into(),
        freq: "7.074".into(),
        mode: "FT8".into(),
        callsign: format!("JA{id}AAA"),
        ..LogEntry::default()
      })
      .collect(),
  })
  .unwrap();
  let exams = serde_json::to_string(&[ExamRecord {
    bank: "A".into(),
    correct: 31,
    total: 40,
    timestamp: 1_700_000_000_000,
    weak: false,
  }])
  .unwrap();
  let sent: std::collections::BTreeMap<String, String> = [
    ("logbook", logbook.as_str()),
    ("exam-history", exams.as_str()),
    ("bookmarks", r#"["A-1","B-2"]"#),
    ("theme", "dark"),
    ("practice:2025-10:A", r#"{"index":5}"#),
  ]
  .into_iter()
  .map(|(k, v)| (k.to_owned(), v.to_owned()))
  .collect();
  let body = serde_json::to_string(&sent).unwrap();

  let saved = put(&app, &cookie, &[], &body).await;
  assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
  let info: BackupInfo = serde_json::from_str(&saved.body).unwrap();
  // 指纹仍是前端那套口径（先去掉设备专属的 `theme` 再求指纹）：拆表没有改变它。
  let mut as_client_builds_it = sent.clone();
  as_client_builds_it.remove("theme");
  assert_eq!(
    info.fingerprint,
    manifest_fingerprint(&manifest_of(&as_client_builds_it, true))
  );

  // 数据确实在各张表里，而不是一整份 JSON。
  let counts: Vec<i64> = svc
    .inner
    .as_ref()
    .unwrap()
    .store
    .with(|c| {
      ["qsos", "exam_results", "bookmarks", "user_kv"]
        .into_iter()
        .map(|t| c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)))
        .collect()
    })
    .unwrap();
  assert_eq!(counts, [3, 1, 2, 5]);

  // 取回来：语义上与上传的完全一致，ETag 与上传时返回的指纹一致。
  let got = get_backup(&app, &cookie).await;
  assert_eq!(got.status, StatusCode::OK);
  let back: std::collections::BTreeMap<String, String> = serde_json::from_str(&got.body).unwrap();
  assert_eq!(back, sent);
  assert_eq!(
    got.headers["etag"].to_str().unwrap(),
    format!("\"{}\"", info.fingerprint.get())
  );

  // 再传一份更小的：库里恰好等于它，没了的 key 与行都被清掉。
  let smaller = r#"{"bookmarks":"[\"A-1\"]"}"#;
  let cond = info.fingerprint.get().to_string();
  let r = put(&app, &cookie, &[("if-match", cond.as_str())], smaller).await;
  assert_eq!(r.status, StatusCode::OK, "{}", r.body);
  let back = get_backup(&app, &cookie).await;
  assert_eq!(
    serde_json::from_str::<std::collections::BTreeMap<String, String>>(&back.body).unwrap(),
    [("bookmarks".to_owned(), r#"["A-1"]"#.to_owned())].into()
  );
}

/// 应用「导出备份」写出的那种文件：整个 localStorage（含设备专属的主题、接口缓存）、
/// IndexedDB 里的日志与 QSL 影像、各类进度。用核心类型生成，与前端写出的字节一致。
fn exported_backup_file() -> std::collections::BTreeMap<String, String> {
  use ham_web_core::bank::Bank;
  use ham_web_core::exam_history::ExamRecord;
  use ham_web_core::logbook::{LogEntry, Logbook};
  use ham_web_core::mistake_book::{MistakeBook, MistakeRecord, StudyStats};
  use ham_web_core::question::{Codes, QuestionItem, QuestionOption, QuestionType};
  use ham_web_core::station::StationBook;

  let question = QuestionItem {
    id: Some("A-1".into()),
    codes: Codes {
      j: Some("J1".into()),
      p: Some("1.1".into()),
    },
    question: "业余无线电台的呼号由谁分配？".into(),
    options: vec![QuestionOption {
      key: "A".into(),
      text: "无线电管理机构".into(),
    }],
    answer_keys: vec!["A".into()],
    kind: QuestionType::Single,
    pages: None,
    image_url: None,
    explanation: None,
  };
  let mistakes = MistakeBook {
    records: vec![MistakeRecord {
      key: "0123456789abcdef".into(),
      question,
      my_answer: vec!["B".into()],
      wrong_count: 2,
      streak: 1,
      last_wrong_ms: 1_700_000_000_000,
      due_ms: 1_700_086_400_000,
      banks: [Bank::A].into(),
      ease: 2.3,
      interval_days: 1.0,
      last_review_ms: 1_700_000_000_000,
      cause: Some("memory".into()),
    }],
  };
  let logbook = Logbook {
    entries: (1..=50)
      .map(|id| LogEntry {
        id,
        station_id: 1,
        date: "2026-03-04".into(),
        time: "12:30".into(),
        freq: "14.074".into(),
        mode: "FT8".into(),
        callsign: format!("BG{id}ABC"),
        remark: "含中文与 \"引号\"".into(),
        ..LogEntry::default()
      })
      .collect(),
  };
  [
    ("logbook", json(&logbook)),
    ("station-book", json(&StationBook::default())),
    ("mistake-book", json(&mistakes)),
    ("study-stats", json(&StudyStats::default())),
    (
      "exam-history",
      json(&vec![ExamRecord {
        bank: "A".into(),
        correct: 33,
        total: 40,
        timestamp: 1_700_000_000_000,
        weak: false,
      }]),
    ),
    ("bookmarks", r#"["A-1","A-7"]"#.to_owned()),
    ("qsl-image-index", "[1]".to_owned()),
    (
      "qsl-image:1",
      format!("data:image/jpeg;base64,{}", "QUJD".repeat(50_000)),
    ),
    (
      "practice:2025-10:A",
      r#"{"index":12,"answers":[]}"#.to_owned(),
    ),
    ("practice:lastMode", "random".to_owned()),
    (
      "learning-plan",
      json(&ham_web_core::study_plan::StudyPlan {
        target_date: "2026-12-01".into(),
        bank: Some(Bank::A),
        reminder_time: "20:00".into(),
      }),
    ),
    // 设备专属 key：手动导出会带上，同步内容里没有，但备份文件里有，必须照单全收。
    ("theme", "dark".to_owned()),
    ("ui:shortcutsHelpSeen:practice", "1".to_owned()),
    ("api-cache:/api/solar", r#"{"t":1}"#.to_owned()),
    // 损坏 / 非 JSON 的值也是用户数据的一部分，不能让整份备份导入失败。
    ("nec-designs-corrupt", "{半截".to_owned()),
    // 页面自己的小文档（类型在 core）、工具与器材、NEC 存档、错题暂存。
    (
      "countdowns",
      json(&ham_web_core::countdown::CountdownList {
        events: vec![ham_web_core::countdown::CountdownEvent {
          id: 1,
          title: "A 类考试".into(),
          target_ms: 1_800_000_000_000,
        }],
      }),
    ),
    (
      "daily-checkin",
      json(&ham_web_core::checkin::CheckinState {
        last_date: "20261105".into(),
        streak: 3,
      }),
    ),
    (
      "morse-stats",
      json(&ham_web_core::morse_store::MorseStats {
        correct: 5,
        mistakes: [("K".to_owned(), 2)].into(),
        daily: [("2026-11-04".to_owned(), (3, 1))].into(),
        ..Default::default()
      }),
    ),
    (
      "morse-trainer",
      json(&ham_web_core::morse_store::TrainerSettings::default()),
    ),
    (
      "listen-settings",
      json(&ham_web_core::listen::Settings::default()),
    ),
    (
      "contest-session",
      json(&ham_web_core::contest::ContestSession::starting(
        "2026-11-05 08:00".into(),
      )),
    ),
    (
      "grid-map-filters",
      json(&ham_web_core::grid_filter::GridFilters::default()),
    ),
    (
      "station-info",
      json(&ham_web_core::logbook::StationInfo::default()),
    ),
    (
      "diy-progress",
      r#"{"天线分析仪":{"done":true,"bought":["外壳"]}}"#.to_owned(),
    ),
    ("exam:wrong:A", "[]".to_owned()),
  ]
  .into_iter()
  .map(|(k, v)| (k.to_owned(), v))
  .collect()
}

fn json<T: serde::Serialize>(value: &T) -> String {
  serde_json::to_string(value).unwrap()
}

#[tokio::test]
async fn an_exported_backup_file_imports_into_the_tables_and_reads_back_identically() {
  let svc = AccountService::in_memory(true);
  let app = svc.clone().router();
  let cookie = signed_up(&app, "alice").await;
  let file = exported_backup_file();
  let body = serde_json::to_string(&file).unwrap();

  let r = put(&app, &cookie, &[], &body).await;
  assert_eq!(r.status, StatusCode::OK, "{}", r.body);

  let count = |table: &str| -> i64 {
    svc
      .inner
      .as_ref()
      .unwrap()
      .store
      .with(|c| c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)))
      .unwrap()
  };
  // 核心数据进了专表……
  assert_eq!(count("qsos"), 50);
  assert_eq!(count("station_profiles"), 1);
  assert_eq!(count("mistakes"), 1);
  assert_eq!(count("exam_results"), 1);
  assert_eq!(count("bookmarks"), 2);
  assert_eq!(count("countdown_events"), 1);
  assert_eq!(count("morse_mistakes"), 1);
  assert_eq!(count("diy_bought"), 1);
  assert_eq!(count("station_info"), 1);
  // ……每个 key（含设备专属的、损坏的、QSL 影像）在目录里恰好一行。
  assert_eq!(count("user_kv"), file.len() as i64);

  // 取回的就是导入的那份，一个 key 不少、一个字节不变。
  let got = get_backup(&app, &cookie).await;
  assert_eq!(got.status, StatusCode::OK);
  let back: std::collections::BTreeMap<String, String> = serde_json::from_str(&got.body).unwrap();
  assert_eq!(back, file);

  // 导入同一份文件两次是幂等的（用户重复点导入、两台设备各导一遍）。
  let again = put(&app, &cookie, &[], &body).await;
  assert_eq!(again.status, StatusCode::OK);
  assert_eq!(count("qsos"), 50);
  assert_eq!(count("user_kv"), file.len() as i64);
}

#[tokio::test]
async fn a_backup_from_an_older_app_version_is_imported_into_the_tables() {
  use ham_web_core::logbook::Logbook;

  let svc = AccountService::in_memory(true);
  let app = svc.clone().router();
  let cookie = signed_up(&app, "alice").await;
  // 旧版导出的文件：日志字段缺失、用旧字段名 `sota_ref`、带已不认识的字段；影像缺 padding、写成 `image/jpg`。
  // 每一样都是合法的备份，必须正确进表——不能因为「和今天的写法不一样」被拒绝，更不能让落库失败。
  let old_logbook = r#"{"entries":[{"id":1,"date":"2024-05-01","time":"08:00","freq":"7.1","mode":"CW","callsign":"JA1AAA","remark":"","sota_ref":"JA/TK-001","retired_field":true}]}"#;
  let file: std::collections::BTreeMap<String, String> = [
    ("logbook", old_logbook),
    ("bookmarks", r#"["A-1"]"#),
    ("qsl-image-index", "[1]"),
    ("qsl-image:1", "data:image/jpg;base64,/9j/4AAQ"),
  ]
  .into_iter()
  .map(|(k, v)| (k.to_owned(), v.to_owned()))
  .collect();

  let put_res = put(&app, &cookie, &[], &serde_json::to_string(&file).unwrap()).await;
  assert_eq!(put_res.status, StatusCode::OK, "{}", put_res.body);
  let info: BackupInfo = serde_json::from_str(&put_res.body).unwrap();

  let store = &svc.inner.as_ref().unwrap().store;
  let count = |t: &str| -> i64 {
    store
      .with(|c| c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0)))
      .unwrap()
  };
  assert_eq!(
    (count("qsos"), count("qsl_images"), count("bookmarks")),
    (1, 1, 1)
  );

  // 取回的是今天的形态：旧字段名认成了新字段，没有的字段补了默认值，影像补齐了 padding、类型规范化。
  let got = get_backup(&app, &cookie).await;
  let back: std::collections::BTreeMap<String, String> = serde_json::from_str(&got.body).unwrap();
  let book: Logbook = serde_json::from_str(&back["logbook"]).unwrap();
  assert_eq!(book.entries[0].my_sota_ref, "JA/TK-001");
  assert_eq!(back["qsl-image:1"], "data:image/jpeg;base64,/9j/4AAQ");

  // 前端靠这一条保持基线：`PUT` 响应里的指纹 == 取回内容按前端口径算出的指纹 == `ETag`。
  assert_eq!(info.fingerprint, backup_fp(&got.body));
  assert_eq!(
    got.headers["etag"].to_str().unwrap(),
    format!("\"{}\"", info.fingerprint.get())
  );

  // 取回的内容再传一遍（前端拉下来之后下一次自动写入就是这样）：内容和指纹都不变，不会越同步越漂。
  let again = put(&app, &cookie, &[], &got.body).await;
  assert_eq!(again.status, StatusCode::OK, "{}", again.body);
  let again: BackupInfo = serde_json::from_str(&again.body).unwrap();
  assert_eq!(again.fingerprint, info.fingerprint);
  assert_eq!(get_backup(&app, &cookie).await.body, got.body);
}

#[tokio::test]
async fn only_well_formed_backups_are_stored() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  for bad in ["not json", "[1,2]", r#"{"a":1}"#, r#"{"a":{"b":"c"}}"#, ""] {
    let r = put(&app, &cookie, &[], bad).await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::BAD_REQUEST, "invalid_backup"),
      "{bad:?}"
    );
  }
  // 空的 `{}` 是合法备份（导出的全新浏览器就是它）。
  assert_eq!(put(&app, &cookie, &[], "{}").await.status, StatusCode::OK);
  // 没声明是 JSON 的请求体不收。
  let r = call(
    &app,
    Method::PUT,
    "/api/account/backup",
    Some(&cookie),
    &[],
    Some(BACKUP),
  )
  .await;
  assert_eq!(r.code(), "bad_request");
}

#[tokio::test]
async fn a_backup_with_an_empty_or_overlong_key_is_refused() {
  use std::collections::BTreeMap;

  use ham_web_core::account::{SYNC_MAX_KEY_LEN, SYNC_MAX_KEYS};

  let app = app();
  let cookie = signed_up(&app, "alice").await;
  let one = |key: String| json(&BTreeMap::from([(key, "x".to_owned())]));
  // 与增量接口同一个口径：key 不能为空、不能超过上限；库里存不下的名字不先收进来。
  let too_many: BTreeMap<String, String> = (0..=SYNC_MAX_KEYS)
    .map(|i| (format!("k{i}"), String::new()))
    .collect();
  for (what, body) in [
    ("空 key", one(String::new())),
    ("超长 key", one("k".repeat(SYNC_MAX_KEY_LEN + 1))),
    ("key 太多", json(&too_many)),
  ] {
    let r = put(&app, &cookie, &[], &body).await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::BAD_REQUEST, "invalid_backup"),
      "{what}"
    );
  }
  // 恰好在上限内的收。
  let edge = one("k".repeat(SYNC_MAX_KEY_LEN));
  assert_eq!(put(&app, &cookie, &[], &edge).await.status, StatusCode::OK);
}

#[tokio::test]
async fn conditional_writes_protect_against_a_device_that_fell_behind() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  let first = backup_fp(BACKUP).get().to_string();

  // 云端为空：`If-None-Match: *` 成功，`If-Match` 失败。
  let stale = put(&app, &cookie, &[("if-match", "123")], BACKUP).await;
  assert_eq!(
    (stale.status, stale.code().as_str()),
    (StatusCode::PRECONDITION_FAILED, "precondition_failed")
  );
  assert_eq!(
    put(&app, &cookie, &[("if-none-match", "*")], BACKUP)
      .await
      .status,
    StatusCode::OK
  );
  // 已有内容：再要求「必须为空」就失败。
  assert_eq!(
    put(&app, &cookie, &[("if-none-match", "*")], BACKUP)
      .await
      .status,
    StatusCode::PRECONDITION_FAILED
  );

  // 设备 B 带着基线写成功；设备 A 还停在旧基线上，被拦下，云端内容没被盖掉。
  let next = r#"{"bookmarks":"[1,2,3]"}"#;
  assert_eq!(
    put(&app, &cookie, &[("if-match", &first)], next)
      .await
      .status,
    StatusCode::OK
  );
  let behind = put(&app, &cookie, &[("if-match", &first)], r#"{"x":"y"}"#).await;
  assert_eq!(behind.code(), "precondition_failed");
  assert_eq!(get_backup(&app, &cookie).await.body, next);
  // 标准写法的带引号 ETag 也认。
  let current = format!("\"{}\"", backup_fp(next).get());
  assert_eq!(
    put(&app, &cookie, &[("if-match", &current)], BACKUP)
      .await
      .status,
    StatusCode::OK
  );
  // 看不懂的条件当错误请求，而不是悄悄变成无条件覆盖。
  let r = put(&app, &cookie, &[("if-match", "banana")], "{}").await;
  assert_eq!(
    (r.status, r.code().as_str()),
    (StatusCode::BAD_REQUEST, "bad_request")
  );
  assert_eq!(get_backup(&app, &cookie).await.body, BACKUP);
}

#[tokio::test]
async fn accounts_never_see_each_others_backups() {
  let app = app();
  let alice = signed_up(&app, "alice").await;
  let bob = signed_up(&app, "bob").await;
  put(&app, &alice, &[], BACKUP).await;
  assert_eq!(get_backup(&app, &bob).await.status, StatusCode::NO_CONTENT);
  put(&app, &bob, &[], r#"{"k":"bob"}"#).await;
  assert_eq!(get_backup(&app, &alice).await.body, BACKUP);
  // 删自己的不会动别人的。
  call(
    &app,
    Method::DELETE,
    "/api/account/backup",
    Some(&bob),
    &[],
    None,
  )
  .await;
  assert_eq!(get_backup(&app, &alice).await.body, BACKUP);
}

#[tokio::test]
async fn a_request_claiming_another_account_is_refused() {
  let app = app();
  let alice = signed_up(&app, "alice").await;
  let id = serde_json::from_str::<SessionInfo>(
    &call(
      &app,
      Method::GET,
      "/api/account/session",
      Some(&alice),
      &[],
      None,
    )
    .await
    .body,
  )
  .unwrap()
  .user
  .unwrap()
  .id;
  let ok = call(
    &app,
    Method::GET,
    "/api/account/backup",
    Some(&alice),
    &[("x-account-id", &id.to_string())],
    None,
  )
  .await;
  assert_eq!(ok.status, StatusCode::NO_CONTENT);
  // 另一个标签页换了账号：旧页面带着旧 id 来，不能写进新账号的云端。
  let other = call(
    &app,
    Method::PUT,
    "/api/account/backup",
    Some(&alice),
    &[JSON, ("x-account-id", &(id + 1).to_string())],
    Some(BACKUP),
  )
  .await;
  assert_eq!(
    (other.status, other.code().as_str()),
    (StatusCode::CONFLICT, "account_changed")
  );
  assert_eq!(
    get_backup(&app, &alice).await.status,
    StatusCode::NO_CONTENT
  );
}

#[tokio::test]
async fn changing_the_password_signs_other_devices_out() {
  let app = app();
  let laptop = signed_up(&app, "alice").await;
  let phone = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "correct horse"),
  )
  .await
  .cookie()
  .unwrap();

  let wrong = post(
    &app,
    "/api/account/password",
    Some(&laptop),
    &serde_json::json!({"current": "nope nope nope", "new": "brand new pass"}).to_string(),
  )
  .await;
  assert_eq!(
    (wrong.status, wrong.code().as_str()),
    (StatusCode::UNAUTHORIZED, "bad_credentials")
  );
  let weak = post(
    &app,
    "/api/account/password",
    Some(&laptop),
    &serde_json::json!({"current": "correct horse", "new": "short"}).to_string(),
  )
  .await;
  assert_eq!(weak.code(), "invalid_password");

  let ok = post(
    &app,
    "/api/account/password",
    Some(&laptop),
    &serde_json::json!({"current": "correct horse", "new": "brand new pass"}).to_string(),
  )
  .await;
  assert_eq!(ok.status, StatusCode::NO_CONTENT);
  // 发起修改的设备仍在线；别的设备被踢下线。
  assert_eq!(
    get_backup(&app, &laptop).await.status,
    StatusCode::NO_CONTENT
  );
  assert_eq!(get_backup(&app, &phone).await.code(), "unauthorized");
  // 旧口令作废，新口令可用。
  assert_eq!(
    post(
      &app,
      "/api/account/login",
      None,
      &creds("alice", "correct horse")
    )
    .await
    .status,
    StatusCode::UNAUTHORIZED
  );
  assert_eq!(
    post(
      &app,
      "/api/account/login",
      None,
      &creds("alice", "brand new pass")
    )
    .await
    .status,
    StatusCode::OK
  );
}

#[tokio::test]
async fn deleting_an_account_removes_everything_and_needs_the_password() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  put(&app, &cookie, &[], BACKUP).await;

  let wrong = post(
    &app,
    "/api/account/delete",
    Some(&cookie),
    r#"{"password":"nope nope nope"}"#,
  )
  .await;
  assert_eq!(wrong.code(), "bad_credentials");
  assert_eq!(get_backup(&app, &cookie).await.status, StatusCode::OK);

  let del = post(
    &app,
    "/api/account/delete",
    Some(&cookie),
    r#"{"password":"correct horse"}"#,
  )
  .await;
  assert_eq!(del.status, StatusCode::NO_CONTENT);
  assert_eq!(del.cookie().as_deref(), Some("ham_session="));
  assert_eq!(get_backup(&app, &cookie).await.code(), "unauthorized");
  assert_eq!(
    post(
      &app,
      "/api/account/login",
      None,
      &creds("alice", "correct horse")
    )
    .await
    .status,
    StatusCode::UNAUTHORIZED
  );
  // 账号名释放出来，可以重新注册，且拿不到旧数据。
  let again = signed_up(&app, "alice").await;
  assert_eq!(
    get_backup(&app, &again).await.status,
    StatusCode::NO_CONTENT
  );
}

// ───────────────────────── 限流：只数失败，分来源 ─────────────────────────

/// 带来源 IP 的登录（测试里没有 `ConnectInfo`，来源只能走 `X-Forwarded-For`）。
async fn login_from(app: &Router, ip: &str, user: &str, pass: &str) -> Reply {
  call(
    app,
    Method::POST,
    "/api/account/login",
    None,
    &[JSON, ("x-forwarded-for", ip)],
    Some(&creds(user, pass)),
  )
  .await
}

#[tokio::test]
async fn correct_logins_never_use_up_the_failure_budget() {
  let app = app();
  signed_up(&app, "alice").await;
  // 远多于失败额度的成功登录：口令对了不耗额度，正常用户不会被自己的勤快挡住。
  for _ in 0..(super::USER_ATTEMPTS * 2) {
    let r = login_from(&app, "198.51.100.1", "alice", "correct horse").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
  }
}

#[tokio::test]
async fn one_source_failing_does_not_lock_the_owner_out_from_elsewhere() {
  let app = app();
  signed_up(&app, "alice").await;
  // 攻击者（来源 A）把自己的额度打光……
  for _ in 0..super::USER_ATTEMPTS {
    let r = login_from(&app, "203.0.113.9", "alice", "wrong password").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
  }
  let blocked = login_from(&app, "203.0.113.9", "alice", "wrong password").await;
  assert_eq!(blocked.code(), "too_many_attempts");
  // ……他自己连正确口令也进不去（否则限流拦不住猜测）……
  let still = login_from(&app, "203.0.113.9", "alice", "correct horse").await;
  assert_eq!(still.code(), "too_many_attempts");
  // ……但本人从别处登录不受影响。
  let owner = login_from(&app, "198.51.100.1", "alice", "correct horse").await;
  assert_eq!(owner.status, StatusCode::OK, "{}", owner.body);
}

#[tokio::test]
async fn guessing_from_many_sources_is_stopped_by_the_per_account_total() {
  let svc = AccountService::in_memory(true);
  let app = svc.router();
  signed_up(&app, "alice").await;
  // 模拟分布式猜测：每个来源都没超额，但这个账号名总共失败得太多了。
  for _ in 0..super::NAME_FAILURES {
    svc.name_failures.record_by_key("alice");
  }
  let r = login_from(&app, "198.51.100.77", "alice", "correct horse").await;
  assert_eq!(r.code(), "too_many_attempts");
  // 别的账号不受牵连。
  signed_up(&app, "bob").await;
}

#[tokio::test]
async fn changing_the_password_shares_the_failure_budget_with_login() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  let wrong = serde_json::json!({"current": "nope nope nope", "new": "brand new pass"}).to_string();
  for _ in 0..super::USER_ATTEMPTS {
    let r = post(&app, "/api/account/password", Some(&cookie), &wrong).await;
    assert_eq!(r.code(), "bad_credentials");
  }
  let r = post(&app, "/api/account/password", Some(&cookie), &wrong).await;
  assert_eq!(r.code(), "too_many_attempts");
  // 同一个来源登录同一个账号，也被同一份额度拦住。
  let login = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "correct horse"),
  )
  .await;
  assert_eq!(login.code(), "too_many_attempts");
}

#[tokio::test]
async fn one_source_cannot_hammer_the_login_endpoint_at_all() {
  let app = app();
  // 超长账号名在查库之前就被当成错误凭据，不耗 Argon2，所以能很快打满 IP 额度。
  let long = "x".repeat(200);
  let mut last = None;
  for _ in 0..(super::IP_ATTEMPTS + 1) {
    last = Some(login_from(&app, "203.0.113.50", &long, "whatever").await);
  }
  assert_eq!(last.unwrap().code(), "too_many_attempts");
  // 别的来源照常。
  let other = login_from(&app, "203.0.113.51", &long, "whatever").await;
  assert_eq!(other.code(), "bad_credentials");
}

#[tokio::test]
async fn absurd_account_names_and_passwords_are_rejected_without_a_lookup() {
  let app = app();
  for (name, pass) in [
    (
      "x".repeat(ham_web_core::account::USERNAME_MAX + 1),
      "correct horse".to_owned(),
    ),
    (
      "alice".to_owned(),
      "p".repeat(ham_web_core::account::PASSWORD_MAX + 1),
    ),
    (String::new(), "correct horse".to_owned()),
  ] {
    let r = post(&app, "/api/account/login", None, &creds(&name, &pass)).await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::UNAUTHORIZED, "bad_credentials")
    );
  }
}

#[tokio::test]
async fn small_json_endpoints_refuse_huge_bodies() {
  let app = app();
  let huge = serde_json::json!({ "username": "alice", "password": "p".repeat(64 * 1024) });
  for uri in ["/api/account/login", "/api/account/register"] {
    let r = post(&app, uri, None, &huge.to_string()).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{uri}");
  }
}

// ───────────────────────── 人数上限 ─────────────────────────

#[tokio::test]
async fn registration_stops_when_the_account_cap_is_reached() {
  let app = AccountService::in_memory_with(AccountConfig {
    max_users: Some(2),
    ..AccountConfig::default()
  })
  .router();
  signed_up(&app, "alice").await;
  signed_up(&app, "bob").await;
  let r = register(&app, "carol", "correct horse").await;
  assert_eq!(
    (r.status, r.code().as_str()),
    (StatusCode::FORBIDDEN, "registration_closed")
  );
  // 已有账号照常登录。
  let l = post(
    &app,
    "/api/account/login",
    None,
    &creds("alice", "correct horse"),
  )
  .await;
  assert_eq!(l.status, StatusCode::OK);
}

// ───────────────────────── 滑动续期要连 Cookie 一起续 ─────────────────────────

/// 直接往库里放一个指定年纪的会话，返回它的 Cookie。
fn session_aged(svc: &AccountService, name: &str, last_seen_ago: i64, expires_in: i64) -> String {
  let inner = svc.inner.clone().unwrap();
  let now = super::handlers::now();
  let user = inner.store.create_user(name, "h", now).unwrap();
  let token = format!("token-for-{name}");
  inner
    .store
    .create_session(
      user.id,
      &super::session::hash_token(&token),
      now - last_seen_ago,
      now + expires_in,
      None,
    )
    .unwrap();
  format!("ham_session={token}")
}

async fn probe(app: &Router, cookie: &str) -> Reply {
  call(
    app,
    Method::GET,
    "/api/account/session",
    Some(cookie),
    &[],
    None,
  )
  .await
}

#[tokio::test]
async fn renewing_a_session_also_renews_the_browser_cookie() {
  let svc = AccountService::in_memory(true);
  let app = svc.router();
  // 两小时没碰过：这次访问会续期，响应要重发 Cookie，让浏览器那边的有效期一起往后挪。
  let stale = session_aged(&svc, "alice", 2 * 3600, 1000);
  let r = probe(&app, &stale).await;
  let set = r
    .headers
    .get("set-cookie")
    .expect("续期要重发 Cookie")
    .to_str()
    .unwrap();
  assert!(set.starts_with(&format!("{stale};")), "同一个令牌：{set}");
  assert!(set.contains(&format!(
    "Max-Age={}",
    AccountConfig::default().session_ttl_secs
  )));
  assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict"));
  // 备份接口上的续期同样重发（页面不一定先问 /session）。
  let stale2 = session_aged(&svc, "bob", 2 * 3600, 1000);
  assert!(
    get_backup(&app, &stale2)
      .await
      .headers
      .contains_key("set-cookie")
  );

  // 刚用过的会话：不续期，也就不重发。
  let fresh = session_aged(&svc, "carol", 60, 1000);
  assert!(!probe(&app, &fresh).await.headers.contains_key("set-cookie"));
}

#[tokio::test]
async fn an_expired_session_is_signed_out_everywhere() {
  let svc = AccountService::in_memory(true);
  let app = svc.router();
  let expired = session_aged(&svc, "alice", 3600, -10);
  let s = probe(&app, &expired).await;
  assert!(
    serde_json::from_str::<SessionInfo>(&s.body)
      .unwrap()
      .user
      .is_none()
  );
  assert!(!s.headers.contains_key("set-cookie"));
  assert_eq!(get_backup(&app, &expired).await.code(), "unauthorized");
}

// ───────────────────────── 备份上传：次序与体积 ─────────────────────────

#[tokio::test]
async fn an_oversized_backup_is_refused_with_413() {
  let app = app();
  let cookie = signed_up(&app, "alice").await;
  let body = format!(
    r#"{{"k":"{}"}}"#,
    "x".repeat(ham_web_core::backup_merge::MAX_BACKUP_BYTES)
  );
  let r = put(&app, &cookie, &[], &body).await;
  assert_eq!(
    (r.status, r.code().as_str()),
    (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
  );
  // 被拒的上传没有留下任何东西。
  assert_eq!(
    get_backup(&app, &cookie).await.status,
    StatusCode::NO_CONTENT
  );
}

#[tokio::test]
async fn an_anonymous_upload_is_turned_away_before_its_body_is_read() {
  let app = app();
  // 声明了超大长度、也没有会话：401 优先，不会因为体积先去读（或先回 413）。
  let r = call(
    &app,
    Method::PUT,
    "/api/account/backup",
    None,
    &[JSON, ("content-length", "999999999")],
    Some("{}"),
  )
  .await;
  assert_eq!(r.code(), "unauthorized");
}

// ───────────────────────── 配置：写错值要报错，不能悄悄放行 ─────────────────────────

fn config_from(pairs: &[(&str, &str)]) -> anyhow::Result<AccountConfig> {
  let map: std::collections::HashMap<String, String> = pairs
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect();
  AccountConfig::from_lookup(|k| map.get(k).cloned())
}

#[test]
fn an_empty_environment_means_the_documented_defaults() {
  let c = config_from(&[]).unwrap();
  assert!(c.db.is_none() && c.registration_open);
  assert_eq!(c.cookie_secure, CookieSecure::Auto);
  assert_eq!(c.session_ttl_secs, 30 * 24 * 3600);
  assert_eq!(c.max_users, Some(super::DEFAULT_MAX_USERS));
  // 空白等于没设。
  assert!(
    config_from(&[("ACCOUNT_REGISTRATION", "  ")])
      .unwrap()
      .registration_open
  );
}

#[test]
fn recognised_values_are_accepted_in_any_case() {
  let c = config_from(&[
    ("ACCOUNT_DB", " /data/a.db "),
    ("ACCOUNT_REGISTRATION", "CLOSED"),
    ("ACCOUNT_COOKIE_SECURE", "TRUE"),
    ("ACCOUNT_SESSION_DAYS", "7"),
    ("ACCOUNT_MAX_USERS", "50"),
  ])
  .unwrap();
  assert_eq!(c.db.as_deref(), Some(std::path::Path::new("/data/a.db")));
  assert!(!c.registration_open);
  assert_eq!(c.cookie_secure, CookieSecure::Always);
  assert_eq!(c.session_ttl_secs, 7 * 24 * 3600);
  assert_eq!(c.max_users, Some(50));
  for off in ["false", "0", "no", "OFF"] {
    assert!(
      !config_from(&[("ACCOUNT_REGISTRATION", off)])
        .unwrap()
        .registration_open,
      "{off}"
    );
  }
  assert_eq!(
    config_from(&[("ACCOUNT_COOKIE_SECURE", "0")])
      .unwrap()
      .cookie_secure,
    CookieSecure::Never
  );
  // 0 = 不限。
  assert_eq!(
    config_from(&[("ACCOUNT_MAX_USERS", "0")])
      .unwrap()
      .max_users,
    None
  );
}

#[test]
fn a_mistyped_value_stops_startup_instead_of_silently_opening_things_up() {
  for (key, bad) in [
    // 少个 d：若被当成「开放」，运维以为关了、实际全网可注册。
    ("ACCOUNT_REGISTRATION", "close"),
    ("ACCOUNT_REGISTRATION", "maybe"),
    ("ACCOUNT_COOKIE_SECURE", "yes please"),
    ("ACCOUNT_SESSION_DAYS", "30d"),
    ("ACCOUNT_SESSION_DAYS", "0"),
    ("ACCOUNT_SESSION_DAYS", "99999"),
    ("ACCOUNT_MAX_USERS", "-1"),
    ("ACCOUNT_MAX_USERS", "lots"),
  ] {
    let err = config_from(&[(key, bad)]).expect_err(&format!("{key}={bad}"));
    assert!(err.to_string().contains(key), "错误里要点名变量：{err}");
  }
}

// ───────────────────────── 按 key 的增量同步 ─────────────────────────

mod delta {
  use super::*;
  use ham_web_core::account::{
    Change, PullRequest, PullResponse, PushRequest, PushResponse, SyncManifest,
  };
  use ham_web_core::delta::Digest;

  async fn manifest(app: &Router, cookie: &str) -> Reply {
    call(
      app,
      Method::GET,
      "/api/account/sync/manifest",
      Some(cookie),
      &[],
      None,
    )
    .await
  }

  async fn pull(app: &Router, cookie: &str, keys: &[&str]) -> Reply {
    let body = serde_json::to_string(&PullRequest {
      keys: keys.iter().map(|k| (*k).to_owned()).collect(),
    })
    .unwrap();
    post(app, "/api/account/sync/pull", Some(cookie), &body).await
  }

  async fn push(app: &Router, cookie: &str, changes: Vec<Change>) -> Reply {
    let body = serde_json::to_string(&PushRequest { changes }).unwrap();
    post(app, "/api/account/sync/push", Some(cookie), &body).await
  }

  fn set(key: &str, base: Option<&str>, value: &str) -> Change {
    Change {
      key: key.to_owned(),
      base: base.map(Digest::of),
      value: Some(value.to_owned()),
    }
  }

  fn remove(key: &str, base: &str) -> Change {
    Change {
      key: key.to_owned(),
      base: Some(Digest::of(base)),
      value: None,
    }
  }

  fn parse<T: serde::de::DeserializeOwned>(r: &Reply) -> T {
    serde_json::from_str(&r.body).unwrap_or_else(|e| panic!("{e}: {}", r.body))
  }

  fn whole(r: &Reply) -> std::collections::BTreeMap<String, String> {
    serde_json::from_str(&r.body).unwrap()
  }

  #[tokio::test]
  async fn an_empty_account_has_no_manifest_and_the_first_push_creates_it() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    assert_eq!(manifest(&app, &cookie).await.status, StatusCode::NO_CONTENT);

    let r = push(
      &app,
      &cookie,
      vec![
        set("bookmarks", None, r#"["A-1"]"#),
        set("plain", None, "x"),
      ],
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let pushed: PushResponse = parse(&r);
    assert_eq!(pushed.keys.len(), 2);
    assert_eq!(pushed.keys["plain"], Digest::of("x"));

    let m: SyncManifest = parse(&manifest(&app, &cookie).await);
    assert_eq!(m.keys, pushed.keys);
    assert_eq!(m.fingerprint, pushed.info.fingerprint);
    // 清单的指纹就是整份接口的 ETag：两个接口共用同一个版本号。
    let all = get_backup(&app, &cookie).await;
    assert_eq!(
      all.headers["etag"].to_str().unwrap(),
      format!("\"{}\"", m.fingerprint.get())
    );
  }

  #[tokio::test]
  async fn two_devices_changing_different_keys_both_succeed() {
    // 这是整套设计要解决的场景：旧协议里设备 B 带着旧基线写，会因为 A 先写过而被 412。
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    let seed = vec![set("bookmarks", None, "[]"), set("practice:x", None, "p0")];
    push(&app, &cookie, seed).await;

    let a = push(
      &app,
      &cookie,
      vec![set("bookmarks", Some("[]"), r#"["A-1"]"#)],
    )
    .await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    // B 没看到 A 的改动（它的 bookmarks 基线还是 "[]"），但它只改 practice:x。
    let b = push(&app, &cookie, vec![set("practice:x", Some("p0"), "p1")]).await;
    assert_eq!(b.status, StatusCode::OK, "{}", b.body);

    let map = whole(&get_backup(&app, &cookie).await);
    assert_eq!(map["bookmarks"], r#"["A-1"]"#);
    assert_eq!(map["practice:x"], "p1");
  }

  #[tokio::test]
  async fn two_devices_changing_the_same_key_conflict_and_the_loser_changes_nothing() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    push(
      &app,
      &cookie,
      vec![set("a", None, "0"), set("b", None, "0")],
    )
    .await;
    let first = push(&app, &cookie, vec![set("a", Some("0"), "A")]).await;
    assert_eq!(first.status, StatusCode::OK);

    // B 以为 a 还是 "0"：被挡下；同一批里它对 b 的改动也一并不生效。
    let loser = vec![set("b", Some("0"), "B"), set("a", Some("0"), "B")];
    let r = push(&app, &cookie, loser).await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::PRECONDITION_FAILED, "precondition_failed")
    );
    let map = whole(&get_backup(&app, &cookie).await);
    assert_eq!((map["a"].as_str(), map["b"].as_str()), ("A", "0"));
  }

  #[tokio::test]
  async fn pulling_returns_values_and_skips_keys_that_are_gone() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    push(
      &app,
      &cookie,
      vec![set("plain", None, "x"), set("gone", None, "y")],
    )
    .await;
    push(&app, &cookie, vec![remove("gone", "y")]).await;
    let r = pull(&app, &cookie, &["plain", "gone", "never"]).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let got: PullResponse = parse(&r);
    assert_eq!(got.values.len(), 1);
    assert_eq!(got.values["plain"], "x");
  }

  #[tokio::test]
  async fn a_non_canonical_value_is_stored_canonically_and_listed_by_its_canonical_digest() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    let raw = r#"{"answered":3}"#;
    let r = push(&app, &cookie, vec![set("study-stats", None, raw)]).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let pushed: PushResponse = parse(&r);
    let got: PullResponse = parse(&pull(&app, &cookie, &["study-stats"]).await);
    let canonical = &got.values["study-stats"];
    assert!(canonical.contains("\"categories\""), "取回的是规范形态");
    // 清单里的摘要是规范文本的，而不是上传原文的：客户端靠基线里的「一对摘要」处理这个差异。
    assert_eq!(pushed.keys["study-stats"], Digest::of(canonical));
    assert_ne!(pushed.keys["study-stats"], Digest::of(raw));
    // 用清单里的摘要当 base 再改：认得上。
    let again = Change {
      key: "study-stats".into(),
      base: Some(pushed.keys["study-stats"]),
      value: Some(canonical.clone()),
    };
    let next = push(&app, &cookie, vec![again]).await;
    assert_eq!(next.status, StatusCode::OK, "{}", next.body);
  }

  #[tokio::test]
  async fn the_account_total_stays_under_the_cap_across_pushes() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    let big = "x".repeat(40 * 1024 * 1024);
    let first = push(&app, &cookie, vec![set("a", None, &big)]).await;
    assert_eq!(first.status, StatusCode::OK);
    // 再来一个 40 MiB：单个请求没超，但加起来超过了账号上限。
    let r = push(&app, &cookie, vec![set("b", None, &big)]).await;
    assert_eq!(
      (r.status, r.code().as_str()),
      (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
    );
    let m: SyncManifest = parse(&manifest(&app, &cookie).await);
    assert_eq!(m.keys.len(), 1, "超限的一批一个都不留");
  }

  #[tokio::test]
  async fn malformed_requests_are_refused_before_anything_is_touched() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    // 同一个 key 两条改动、空 key、超长 key：都是格式错误。
    let dup = push(
      &app,
      &cookie,
      vec![set("k", None, "1"), set("k", None, "2")],
    )
    .await;
    assert_eq!(dup.code(), "bad_request");
    assert_eq!(
      push(&app, &cookie, vec![set("", None, "1")]).await.code(),
      "bad_request"
    );
    let long = "k".repeat(513);
    assert_eq!(
      push(&app, &cookie, vec![set(&long, None, "1")])
        .await
        .code(),
      "bad_request"
    );
    let junk = r#"{"changes":[{"key":1}]}"#;
    let r = post(&app, "/api/account/sync/push", Some(&cookie), junk).await;
    assert_eq!(r.code(), "bad_request");
    assert_eq!(pull(&app, &cookie, &["a", "a"]).await.code(), "bad_request");
    assert_eq!(
      manifest(&app, &cookie).await.status,
      StatusCode::NO_CONTENT,
      "什么都没写进去"
    );
  }

  #[tokio::test]
  async fn an_oversized_pull_body_is_refused_after_authentication() {
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    // 一个 key 名撑到比上限还大：登录之后才读体，读到上限就拒绝。
    let huge = format!(
      r#"{{"keys":["{}"]}}"#,
      "k".repeat(crate::account::PULL_BODY_LIMIT)
    );
    let r = post(&app, "/api/account/sync/pull", Some(&cookie), &huge).await;
    assert_eq!(r.code(), "too_large");
    // 没登录的同一个请求回 401，不是 413：鉴权在读体之前。
    let r = call(
      &app,
      Method::POST,
      "/api/account/sync/pull",
      None,
      &[JSON],
      Some(&huge),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn the_endpoints_need_a_session_json_and_the_right_account() {
    let app = app();
    for (method, uri) in [
      (Method::GET, "/api/account/sync/manifest"),
      (Method::POST, "/api/account/sync/pull"),
      (Method::POST, "/api/account/sync/push"),
    ] {
      let r = call(&app, method, uri, None, &[JSON], Some("{}")).await;
      assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let cookie = signed_up(&app, "alice").await;
    let uri = "/api/account/sync/push";
    // 不是 JSON：拒绝（第二道 CSRF 防线）。
    let r = call(&app, Method::POST, uri, Some(&cookie), &[], Some("{}")).await;
    assert_eq!(r.code(), "bad_request");
    // 声称是另一个账号：拒绝，不把 A 的改动写进 B 的云端。
    let claim = [JSON, ("x-account-id", "999")];
    let r = call(
      &app,
      Method::POST,
      uri,
      Some(&cookie),
      &claim,
      Some(r#"{"changes":[]}"#),
    )
    .await;
    assert_eq!(r.code(), "account_changed");
  }

  #[tokio::test]
  async fn accounts_never_see_each_others_keys() {
    let app = app();
    let alice = signed_up(&app, "alice").await;
    let bob = signed_up(&app, "bobby").await;
    push(&app, &alice, vec![set("secret", None, "alice's")]).await;
    assert_eq!(manifest(&app, &bob).await.status, StatusCode::NO_CONTENT);
    assert!(
      parse::<PullResponse>(&pull(&app, &bob, &["secret"]).await)
        .values
        .is_empty()
    );
    // bob 以为没有（None）是对的：他写同名 key 互不影响。
    let r = push(&app, &bob, vec![set("secret", None, "bob's")]).await;
    assert_eq!(r.status, StatusCode::OK);
    let a: PullResponse = parse(&pull(&app, &alice, &["secret"]).await);
    assert_eq!(a.values["secret"], "alice's");
  }

  #[tokio::test]
  async fn the_whole_file_and_delta_endpoints_share_one_version_number() {
    // 手动「上传到云端」走整份接口，自动同步走增量接口：同一份内容，两边看到的版本号必须一样，
    // 否则自动同步每次都会把手动上传的结果当成「云端被别处改了」。
    let app = app();
    let cookie = signed_up(&app, "alice").await;
    let r = put(&app, &cookie, &[], BACKUP).await;
    let whole: BackupInfo = serde_json::from_str(&r.body).unwrap();
    // 整份上传的回执里带着写入之后的清单，客户端据此建立基线，不必再问一次。
    let receipt: ham_web_core::account::BackupReceipt = serde_json::from_str(&r.body).unwrap();
    assert_eq!(receipt.info, whole);
    assert_eq!(
      receipt.keys.keys().collect::<Vec<_>>(),
      ["bookmarks", "logbook"]
    );
    let m: SyncManifest = parse(&manifest(&app, &cookie).await);
    assert_eq!(m.fingerprint, whole.fingerprint);
    // 增量接口把云端改成别的、再改回同样的内容，指纹也回到同一个值。
    push(&app, &cookie, vec![set("bookmarks", Some("[1,2]"), "[3]")]).await;
    let back = push(&app, &cookie, vec![set("bookmarks", Some("[3]"), "[1,2]")]).await;
    assert_eq!(
      parse::<PushResponse>(&back).info.fingerprint,
      whole.fingerprint
    );
  }
}
