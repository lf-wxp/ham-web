//! `/api/account/*` 的处理函数。
//!
//! 约定：
//! - 所有会碰数据库 / 做哈希的活都放进 `spawn_blocking`（见 [`Inner::db`] / [`Inner::hash`]）；
//! - 请求体一律要求 `Content-Type: application/json`：浏览器的跨站表单 / `sendBeacon`
//!   发不出这个类型而不触发预检，这是 `SameSite=Strict` 之外的第二道 CSRF 防线；
//! - 登录失败不区分「账号不存在」与「口令不对」（响应与耗时都一样，见 `password::verify_dummy`）；
//! - 猜口令的限流**只数失败**（登录 / 改口令 / 注销共用），分「账号名 + 来源 IP」与「账号名」两层
//!   （额度见 `account/mod.rs`）：口令对了不耗额度，别人也没法靠几次失败就把本人锁在门外。

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Extension, Request, State};
use axum::http::header::{CONTENT_TYPE, ETAG, SET_COOKIE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ham_web_core::account::{
  ACCOUNT_ID_HEADER, ChangePassword, ConfirmPassword, Credentials, ErrorCode, PASSWORD_MAX,
  Precondition, SessionInfo, USERNAME_MAX, UserInfo, validate_password, validate_username,
};
use ham_web_core::backup_merge::backup_size_is_supported;
use ham_web_core::remote_sync::fingerprint_json;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::error::ApiError;
use super::store::{SessionHit, Store, StoreError, UserRow};
use super::{AccountService, BODY_LIMIT, CookieSecure, Inner, password, session};
use crate::rate_limit;

type ApiResult<T = Response> = Result<T, ApiError>;

/// 当前 Unix 秒。
pub fn now() -> i64 {
  chrono::Utc::now().timestamp()
}

fn internal(what: &str, e: impl std::fmt::Debug) -> ApiError {
  tracing::error!("{what}：{e:?}");
  ApiError::new(ErrorCode::Internal)
}

async fn blocking<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> ApiResult<R> {
  tokio::task::spawn_blocking(f)
    .await
    .map_err(|e| internal("阻塞任务失败", e))
}

impl AccountService {
  fn inner(&self) -> ApiResult<&Arc<Inner>> {
    self
      .inner
      .as_ref()
      .ok_or(ApiError::new(ErrorCode::Unavailable))
  }

  /// 「账号名 + 来源」的限流 key。认不出来源（如本机联调）时用 `-` 占位，仍然按账号名限。
  fn pair_key(name: &str, ip: ClientIp) -> String {
    let who = ip.0.map_or_else(|| "-".to_owned(), |i| i.to_string());
    format!("{}|{who}", name.to_ascii_lowercase())
  }

  /// 这个来源对该账号的口令尝试是否已被限流（只看不记）。
  fn guesses_blocked(&self, name: &str, ip: ClientIp) -> bool {
    self
      .user_attempts
      .is_blocked_by_key(&Self::pair_key(name, ip))
      || self
        .name_failures
        .is_blocked_by_key(&name.to_ascii_lowercase())
  }

  /// 记一次口令失败（账号存不存在都记，免得限流本身泄露账号是否存在）。
  fn note_bad_guess(&self, name: &str, ip: ClientIp) {
    self.user_attempts.record_by_key(&Self::pair_key(name, ip));
    self.name_failures.record_by_key(&name.to_ascii_lowercase());
  }

  /// 口令对了：清掉这个来源的失败记录。账号名维度的总数不清——那是挡分布式猜测的，
  /// 攻击者不能靠夹几次自己的成功登录来重置它。
  fn note_good_guess(&self, name: &str, ip: ClientIp) {
    self.user_attempts.clear_by_key(&Self::pair_key(name, ip));
  }
}

/// 请求的来源 IP（由 [`tag_client_ip`] 解析后放进请求扩展）；认不出来为 `None`。
#[derive(Debug, Clone, Copy)]
pub struct ClientIp(Option<IpAddr>);

/// 解析来源 IP 并挂到请求上，让后面的处理函数不必各自拿 `ConnectInfo` 再算一遍。
pub async fn tag_client_ip(mut req: Request, next: Next) -> Response {
  let conn = req.extensions().get::<ConnectInfo<SocketAddr>>().cloned();
  let ip = rate_limit::client_ip(conn, &req);
  req.extensions_mut().insert(ClientIp(ip));
  next.run(req).await
}

impl Inner {
  /// 在阻塞线程里跑一段数据库操作。
  async fn db<R: Send + 'static>(
    &self,
    f: impl FnOnce(&Store) -> Result<R, StoreError> + Send + 'static,
  ) -> ApiResult<R> {
    let store = self.store.clone();
    blocking(move || f(&store)).await?.map_err(ApiError::from)
  }

  /// 哈希口令（占一个并发名额）。
  async fn hash(&self, password: String) -> ApiResult<String> {
    let _slot = self
      .hash_slots
      .acquire()
      .await
      .map_err(|e| internal("哈希名额", e))?;
    blocking(move || password::hash(&password))
      .await?
      .map_err(|e| internal("口令哈希", e))
  }

  /// 校验口令；`phc` 为 `None`（账号不存在）时做一次等价的空校验并返回 `false`。
  async fn verify(&self, password: String, phc: Option<String>) -> ApiResult<bool> {
    let _slot = self
      .hash_slots
      .acquire()
      .await
      .map_err(|e| internal("哈希名额", e))?;
    blocking(move || match phc {
      Some(phc) => password::verify(&password, &phc),
      None => {
        password::verify_dummy(&password);
        false
      }
    })
    .await
  }

  fn secure(&self, headers: &HeaderMap) -> bool {
    match self.cfg.cookie_secure {
      CookieSecure::Always => true,
      CookieSecure::Never => false,
      CookieSecure::Auto => headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https")),
    }
  }

  async fn user_info(&self, user: &UserRow) -> ApiResult<UserInfo> {
    let id = user.id;
    let backup = self.db(move |s| s.backup_info(id)).await?;
    Ok(UserInfo {
      id,
      username: user.username.clone(),
      created_at: user.created_at,
      backup,
    })
  }

  fn session_info(&self, user: Option<UserInfo>) -> SessionInfo {
    SessionInfo {
      enabled: true,
      registration_open: self.cfg.registration_open,
      user,
    }
  }

  /// 给用户开一个会话，返回要下发的 `Set-Cookie`。
  async fn start_session(&self, user: &UserRow, headers: &HeaderMap) -> ApiResult<String> {
    let token = session::new_token();
    let hash = session::hash_token(&token);
    let ttl = self.cfg.session_ttl_secs;
    let (id, now) = (user.id, now());
    let ua = headers
      .get(axum::http::header::USER_AGENT)
      .and_then(|v| v.to_str().ok())
      // 只作排查线索：截断，免得被塞进超长的值。
      .map(|v| v.chars().take(200).collect::<String>());
    self
      .db(move |s| {
        // 顺手清过期会话：登录是写入路径，不增加额外的定时依赖。
        s.purge_expired_sessions(now)?;
        s.create_session(id, &hash, now, now + ttl, ua.as_deref())
      })
      .await?;
    Ok(session::set_cookie(&token, ttl, self.secure(headers)))
  }
}

/// 已通过会话校验的请求者。
struct Authed {
  user: UserRow,
  token: String,
  token_hash: [u8; 32],
  /// 这次访问把会话续了期（见 [`SessionHit::renewed`]）。
  renewed: bool,
}

impl Authed {
  /// 续了期就返回要重发的 `Set-Cookie`：库里的有效期往后挪了，浏览器那份 Cookie 的 `Max-Age`
  /// 也得跟着挪，否则天天在用的人到了登录那天起算的 N 天后照样被登出，「滑动续期」名不副实。
  fn renewal_cookie(&self, inner: &Inner, headers: &HeaderMap) -> Option<String> {
    self.renewed.then(|| {
      session::set_cookie(
        &self.token,
        inner.cfg.session_ttl_secs,
        inner.secure(headers),
      )
    })
  }
}

/// 给响应补上续期 Cookie（没有就原样返回）。
fn with_renewal(res: Response, cookie: Option<String>) -> Response {
  match cookie {
    Some(c) => with_cookie(res, &c),
    None => res,
  }
}

/// 从 Cookie 识别请求者；带了 `X-Account-Id` 时还要核对是不是同一个账号。
async fn authenticate(inner: &Inner, headers: &HeaderMap) -> ApiResult<Authed> {
  let token = session::token_from_headers(headers).ok_or(ApiError::new(ErrorCode::Unauthorized))?;
  let token_hash = session::hash_token(&token);
  let (now, ttl) = (now(), inner.cfg.session_ttl_secs);
  let SessionHit { user, renewed } = inner
    .db(move |s| s.find_session(&token_hash, now, ttl))
    .await?
    .ok_or(ApiError::new(ErrorCode::Unauthorized))?;
  // 两个标签页登录了不同账号时，旧标签页的自动同步会带着自己认定的账号 id 来：对不上就拒绝，
  // 否则它会把 A 的数据写进 B 的云端。
  if let Some(claimed) = headers.get(ACCOUNT_ID_HEADER).and_then(|v| v.to_str().ok())
    && claimed.trim().parse::<u64>().ok() != Some(user.id)
  {
    return Err(ApiError::new(ErrorCode::AccountChanged));
  }
  Ok(Authed {
    user,
    token,
    token_hash,
    renewed,
  })
}

fn require_json(headers: &HeaderMap) -> ApiResult<()> {
  let ok = headers
    .get(CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .is_some_and(|v| {
      v.trim()
        .to_ascii_lowercase()
        .starts_with("application/json")
    });
  if ok {
    Ok(())
  } else {
    Err(ApiError::new(ErrorCode::BadRequest))
  }
}

fn parse_json<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> ApiResult<T> {
  require_json(headers)?;
  serde_json::from_slice(body).map_err(|_| ApiError::new(ErrorCode::BadRequest))
}

fn json<T: Serialize>(status: StatusCode, body: &T) -> Response {
  let text = serde_json::to_string(body).unwrap_or_else(|_| "null".to_owned());
  (status, [(CONTENT_TYPE, "application/json")], text).into_response()
}

fn with_cookie(mut res: Response, cookie: &str) -> Response {
  if let Ok(v) = HeaderValue::from_str(cookie) {
    res.headers_mut().append(SET_COOKIE, v);
  }
  res
}

/// 登录 / 注册前的限流：按 IP，成败都算。猜口令的限流（只数失败）在各处理函数里，
/// 那时才知道账号名。
pub async fn limit_attempts(
  State(svc): State<AccountService>,
  req: Request,
  next: Next,
) -> Response {
  let conn = req.extensions().get::<ConnectInfo<SocketAddr>>().cloned();
  if rate_limit::allow(&svc.ip_attempts, conn, &req) {
    next.run(req).await
  } else {
    ApiError::new(ErrorCode::TooManyAttempts).into_response()
  }
}

/// `GET /api/account/session`：探测登录态。**永远 200**（未登录是 `user: null`），
/// 页面每次打开都会问一次，用 401 会在控制台里留一条红色错误。
pub async fn session(State(svc): State<AccountService>, headers: HeaderMap) -> ApiResult {
  let Some(inner) = svc.inner.as_ref() else {
    return Ok(json(
      StatusCode::OK,
      &SessionInfo {
        enabled: false,
        registration_open: false,
        user: None,
      },
    ));
  };
  let (user, renewal) = match authenticate(inner, &headers).await {
    Ok(a) => (
      Some(inner.user_info(&a.user).await?),
      a.renewal_cookie(inner, &headers),
    ),
    Err(e) if matches!(e.code, ErrorCode::Unauthorized) => (None, None),
    Err(e) => return Err(e),
  };
  Ok(with_renewal(
    json(StatusCode::OK, &inner.session_info(user)),
    renewal,
  ))
}

/// `POST /api/account/register`
pub async fn register(
  State(svc): State<AccountService>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult {
  let inner = svc.inner()?;
  if !inner.cfg.registration_open {
    return Err(ApiError::new(ErrorCode::RegistrationClosed));
  }
  let creds: Credentials = parse_json(&headers, &body)?;
  let username = validate_username(&creds.username)?;
  validate_password(&creds.password, &username)?;

  let phc = inner.hash(creds.password).await?;
  let (name, now, cap) = (username.clone(), now(), inner.cfg.max_users);
  let user = inner
    .db(move |s| s.create_user_capped(&name, &phc, now, cap))
    .await?;
  let cookie = inner.start_session(&user, &headers).await?;
  let info = inner.session_info(Some(inner.user_info(&user).await?));
  Ok(with_cookie(json(StatusCode::CREATED, &info), &cookie))
}

/// `POST /api/account/login`
pub async fn login(
  State(svc): State<AccountService>,
  Extension(ip): Extension<ClientIp>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult {
  let inner = svc.inner()?;
  let creds: Credentials = parse_json(&headers, &body)?;
  let name = creds.username.trim().to_owned();
  // 超长的账号名 / 口令不可能是任何一个账号的凭据：直接当错误凭据，
  // 不为它跑一次 Argon2、查一次库，更不让它占一个限流桶（key 的长度由攻击者决定）。
  let bad = ApiError::new(ErrorCode::BadCredentials);
  if name.is_empty() || name.len() > USERNAME_MAX || creds.password.len() > PASSWORD_MAX {
    return Err(bad);
  }
  if svc.guesses_blocked(&name, ip) {
    return Err(ApiError::new(ErrorCode::TooManyAttempts));
  }
  let lookup = name.clone();
  let user = inner.db(move |s| s.find_user_by_name(&lookup)).await?;
  let matched = inner
    .verify(
      creds.password,
      user.as_ref().map(|u| u.password_hash.clone()),
    )
    .await?;
  let Some(user) = user.filter(|_| matched) else {
    svc.note_bad_guess(&name, ip);
    return Err(bad);
  };
  svc.note_good_guess(&name, ip);
  let cookie = inner.start_session(&user, &headers).await?;
  let info = inner.session_info(Some(inner.user_info(&user).await?));
  Ok(with_cookie(json(StatusCode::OK, &info), &cookie))
}

/// `POST /api/account/logout`：幂等，没登录也是 204。
pub async fn logout(State(svc): State<AccountService>, headers: HeaderMap) -> ApiResult {
  let mut secure = false;
  if let Some(inner) = svc.inner.as_ref() {
    secure = inner.secure(&headers);
    if let Some(token) = session::token_from_headers(&headers) {
      let hash = session::hash_token(&token);
      inner.db(move |s| s.delete_session(&hash)).await?;
    }
  }
  Ok(with_cookie(
    StatusCode::NO_CONTENT.into_response(),
    &session::clear_cookie(secure),
  ))
}

/// `POST /api/account/password`：改口令，并让其他设备的登录失效。
pub async fn change_password(
  State(svc): State<AccountService>,
  Extension(ip): Extension<ClientIp>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult {
  let inner = svc.inner()?;
  let auth = authenticate(inner, &headers).await?;
  let req: ChangePassword = parse_json(&headers, &body)?;
  // 改口令也是一次口令猜测的机会（偷到会话的人想知道原口令）：与登录共用「只数失败」的限流。
  let name = auth.user.username.clone();
  if svc.guesses_blocked(&name, ip) {
    return Err(ApiError::new(ErrorCode::TooManyAttempts));
  }
  validate_password(&req.new, &name)?;
  if req.current.len() > PASSWORD_MAX
    || !inner
      .verify(req.current, Some(auth.user.password_hash.clone()))
      .await?
  {
    svc.note_bad_guess(&name, ip);
    return Err(ApiError::new(ErrorCode::BadCredentials));
  }
  svc.note_good_guess(&name, ip);
  let phc = inner.hash(req.new).await?;
  let (id, keep, now) = (auth.user.id, auth.token_hash, now());
  inner
    .db(move |s| {
      s.set_password_hash(id, &phc, now)?;
      s.delete_other_sessions(id, &keep)
    })
    .await?;
  Ok(with_renewal(
    StatusCode::NO_CONTENT.into_response(),
    auth.renewal_cookie(inner, &headers),
  ))
}

/// `POST /api/account/delete`：注销账号（再确认一次口令）；云端备份一并删除。
pub async fn delete_account(
  State(svc): State<AccountService>,
  Extension(ip): Extension<ClientIp>,
  headers: HeaderMap,
  body: Bytes,
) -> ApiResult {
  let inner = svc.inner()?;
  let auth = authenticate(inner, &headers).await?;
  let req: ConfirmPassword = parse_json(&headers, &body)?;
  let name = auth.user.username.clone();
  if svc.guesses_blocked(&name, ip) {
    return Err(ApiError::new(ErrorCode::TooManyAttempts));
  }
  if req.password.len() > PASSWORD_MAX
    || !inner
      .verify(req.password, Some(auth.user.password_hash.clone()))
      .await?
  {
    svc.note_bad_guess(&name, ip);
    return Err(ApiError::new(ErrorCode::BadCredentials));
  }
  svc.note_good_guess(&name, ip);
  let id = auth.user.id;
  inner.db(move |s| s.delete_user(id)).await?;
  Ok(with_cookie(
    StatusCode::NO_CONTENT.into_response(),
    &session::clear_cookie(inner.secure(&headers)),
  ))
}

/// `GET /api/account/backup`：云端备份原文；还没有为 `204`。
pub async fn get_backup(State(svc): State<AccountService>, headers: HeaderMap) -> ApiResult {
  let inner = svc.inner()?;
  let auth = authenticate(inner, &headers).await?;
  let renewal = auth.renewal_cookie(inner, &headers);
  let id = auth.user.id;
  let Some((body, info)) = inner.db(move |s| s.get_backup(id)).await? else {
    return Ok(with_renewal(
      StatusCode::NO_CONTENT.into_response(),
      renewal,
    ));
  };
  let mut res = (StatusCode::OK, [(CONTENT_TYPE, "application/json")], body).into_response();
  if let Ok(v) = HeaderValue::from_str(&format!("\"{}\"", info.fingerprint.get())) {
    res.headers_mut().insert(ETAG, v);
  }
  if let Ok(v) = HeaderValue::from_str(&info.updated_at.to_string()) {
    res
      .headers_mut()
      .insert(HeaderName::from_static("x-backup-updated-at"), v);
  }
  Ok(with_renewal(res, renewal))
}

/// `PUT /api/account/backup`：整体替换云端备份。
///
/// 带 `If-Match: <指纹>` 时，只有云端现在的指纹等于它才写（否则 `412`）；带 `If-None-Match: *`
/// 时，只有云端还没有备份才写。两个头都没有 = 无条件覆盖（用户手动「上传」）。
/// 检查与写入在同一个事务里，两台设备不会同时「赢」。
///
/// 次序有讲究（请求体最大 64 MiB）：先鉴权、先校验头，再拿上传名额，**最后**才读请求体——
/// 没登录的请求读不到一个字节；同时在内存里的请求体不超过 `UPLOAD_SLOTS` 份；
/// 解析与算指纹是 CPU 重活，放进 `spawn_blocking`，不占异步运行时线程。
pub async fn put_backup(State(svc): State<AccountService>, req: Request) -> ApiResult {
  let inner = svc.inner()?;
  let (parts, body) = req.into_parts();
  let headers = parts.headers;
  let auth = authenticate(inner, &headers).await?;
  require_json(&headers)?;
  let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
  let pre = Precondition::from_headers(header("if-match"), header("if-none-match"))
    .map_err(|_| ApiError::new(ErrorCode::BadRequest))?;
  // 声明的长度就超限的，不必占名额、不必读。
  let declared = header("content-length").and_then(|v| v.trim().parse::<usize>().ok());
  if declared.is_some_and(|n| n > BODY_LIMIT) {
    return Err(ApiError::new(ErrorCode::TooLarge));
  }
  let _slot = inner
    .upload_slots
    .acquire()
    .await
    .map_err(|e| internal("上传名额", e))?;
  // 读失败有两种：超过上限，或连接中途断了——后者没人在等响应，不必区分。
  let body = axum::body::to_bytes(body, BODY_LIMIT)
    .await
    .map_err(|_| ApiError::new(ErrorCode::TooLarge))?;
  if !backup_size_is_supported(body.len()) {
    return Err(ApiError::new(ErrorCode::TooLarge));
  }
  // 内容必须是 `{ key: value }` 的字符串表：与导入的校验口径一致，云端不存任何恢复不了的东西。
  let (body, fingerprint) = blocking(move || {
    let fp = std::str::from_utf8(&body).ok().and_then(fingerprint_json);
    (body, fp)
  })
  .await?;
  let fingerprint = fingerprint.ok_or(ApiError::new(ErrorCode::InvalidBackup))?;
  let (id, now) = (auth.user.id, now());
  let info = inner
    .db(move |s| s.put_backup(id, &body, fingerprint, now, pre))
    .await?;
  Ok(with_renewal(
    json(StatusCode::OK, &info),
    auth.renewal_cookie(inner, &headers),
  ))
}

/// `DELETE /api/account/backup`：清掉云端备份。
pub async fn delete_backup(State(svc): State<AccountService>, headers: HeaderMap) -> ApiResult {
  let inner = svc.inner()?;
  let auth = authenticate(inner, &headers).await?;
  let id = auth.user.id;
  inner.db(move |s| s.delete_backup(id)).await?;
  Ok(with_renewal(
    StatusCode::NO_CONTENT.into_response(),
    auth.renewal_cookie(inner, &headers),
  ))
}
