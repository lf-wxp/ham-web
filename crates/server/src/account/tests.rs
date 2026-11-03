//! 账号接口的端到端测试：走完整的路由与中间件，数据库用内存库。

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use ham_web_core::account::{BackupInfo, SessionInfo};
use ham_web_core::remote_sync::{Fingerprint, fingerprint};
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

fn backup_fp(json: &str) -> Fingerprint {
  let map: std::collections::BTreeMap<String, String> = serde_json::from_str(json).unwrap();
  fingerprint(&map)
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
  assert_eq!(info.size as usize, BACKUP.len());

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
