//! 在线账号的浏览器侧：登录态、账号操作，以及作为一条同步通道的「获取 / 写入」。
//!
//! 它与 `crate::remote_sync`（自定义远端）是**并列**的两条通道：交换同一份备份 JSON
//! （不含设备专属 key），共用同一套请求超时、错误分类与体积检查；去处不同——这里是同源的
//! `/api/account/backup`，鉴权靠 `HttpOnly` 会话 Cookie（页面脚本碰不到令牌）。
//! 这里只管**手动**的「上传 / 恢复 / 合并」（整份内容，用户明确要覆盖）；**自动**同步按 key 增量，
//! 在 `crate::account_cloud`（接口）与 `ham_web_core::delta_sync`（算法）。
//! 两条通道怎么共存见 `docs/domain/account-sync.md`。
//!
//! 页面里拿到的 JSON 一律用 `serde_json` 解析，不用 `Response.json()`：指纹是 `u64`，
//! 超过 2^53 的数走 JS 的 `JSON.parse` 会丢精度。

use std::cell::Cell;
use std::collections::BTreeMap;

use ham_web_core::account::{
  ACCOUNT_ID_HEADER, API_BASE, BackupReceipt, ChangePassword, ConfirmPassword, Credentials,
  ErrorCode, Precondition, SessionInfo, UserInfo,
};
use ham_web_core::delta::{
  Manifest, base_after_pull, base_after_push, manifest_fingerprint, manifest_of,
};
use ham_web_core::sync_channel::{ACCOUNT_OPTIONS_KEY, AccountOptions};
use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::i18n::{t, tf};
use crate::remote_sync::{
  Fetched, PullResult, PushError, Pushed, SyncError, import_fetched, request,
};
use crate::util::{BackupModules, storage};

/// 单次请求的总超时：与自定义远端一致，备份可达几十 MB。
pub(crate) const TIMEOUT_MS: i32 = 60_000;

/// 面向界面的登录态。
#[derive(Clone, Debug, PartialEq)]
pub enum AccountState {
  /// 还没问过服务器。
  Unknown,
  /// 服务器没启用在线账号（没配 `ACCOUNT_DB`），或根本没有后端（纯静态托管）。
  Unavailable,
  /// 已启用、没登录。
  SignedOut {
    /// 是否开放注册。
    registration_open: bool,
  },
  /// 已登录。
  SignedIn(UserInfo),
}

thread_local! {
  static STATE: ArcRwSignal<AccountState> = ArcRwSignal::new(AccountState::Unknown);
  /// 登录态的「版本」：每次落定新的登录态（登录 / 退出 / 刷新结果）加一。
  /// 在路上的 `refresh` 回来先比一下，不一致说明有更新的结论已经落地，它的答案就过时了。
  static EPOCH: Cell<u64> = const { Cell::new(0) };
}

/// 登录态（响应式，给面板展示）。
pub fn state() -> ArcRwSignal<AccountState> {
  STATE.with(Clone::clone)
}

/// 当前登录账号的 id；没登录为 `None`。
pub fn current_user_id() -> Option<u64> {
  STATE.with(|s| match s.get_untracked() {
    AccountState::SignedIn(u) => Some(u.id),
    _ => None,
  })
}

fn set_state(next: AccountState) {
  EPOCH.with(|e| e.set(e.get().wrapping_add(1)));
  STATE.with(|s| s.set(next));
}

// ───────────────────────── 选项 ─────────────────────────

/// 账号通道的同步选项；没存过 / 读不出来就是默认（同步全部内容）。
pub fn options() -> AccountOptions {
  storage::get_json(ACCOUNT_OPTIONS_KEY).unwrap_or_default()
}

/// 保存选项。
pub fn save_options(options: &AccountOptions) {
  storage::set_json(ACCOUNT_OPTIONS_KEY, options);
}

/// 账号通道是否同步 QSL 影像。
pub fn include_images() -> bool {
  !options().exclude_qsl_images
}

fn modules() -> BackupModules {
  BackupModules::for_sync(include_images())
}

// ───────────────────────── HTTP ─────────────────────────

/// 发一次账号接口请求。
///
/// `claim_account`：带上 `X-Account-Id`，让服务端核对「当前登录的就是我以为的那个账号」——
/// 另一个标签页换了账号时，这个页面的自动同步不能把数据写进别人的云端。
async fn call(
  method: &str,
  path: &str,
  json_body: Option<String>,
  claim_account: bool,
  precondition: Precondition,
) -> Result<(u16, String), SyncError> {
  call_within(
    method,
    path,
    json_body,
    claim_account,
    precondition,
    TIMEOUT_MS,
  )
  .await
}

/// [`call`] 的可调超时版本（启动时的自动获取不能被一个挂住的服务拖太久）。
pub(crate) async fn call_within(
  method: &str,
  path: &str,
  json_body: Option<String>,
  claim_account: bool,
  precondition: Precondition,
  timeout_ms: i32,
) -> Result<(u16, String), SyncError> {
  let mut headers: Vec<(String, String)> = Vec::new();
  if json_body.is_some() {
    headers.push(("Content-Type".into(), "application/json".into()));
  }
  if claim_account && let Some(id) = current_user_id() {
    headers.push((ACCOUNT_ID_HEADER.into(), id.to_string()));
  }
  if let Some((name, value)) = precondition.header() {
    headers.push((name.into(), value));
  }
  let body = json_body.map(|b| JsValue::from_str(&b));
  request(
    method,
    &format!("{API_BASE}{path}"),
    &headers,
    body,
    timeout_ms,
  )
  .await
}

/// 非 2xx 的响应：能解析出错误码就用码，否则按 HTTP 状态码。
pub(crate) fn failure(status: u16, body: &str) -> SyncError {
  let code = serde_json::from_str::<serde_json::Value>(body)
    .ok()
    .and_then(|v| v["error"]["code"].as_str().and_then(ErrorCode::parse));
  code.map_or(SyncError::Http(status), SyncError::Account)
}

pub(crate) fn is_success(status: u16) -> bool {
  (200..300).contains(&status)
}

/// 账号通道里一次请求失败的界面文案。
///
/// 网络错误与体积超限的通用文案是写给自定义远端的（提示去查地址、跨域，说"远端"）；
/// 账号接口是同源的，要换成"云端"和对应的建议。其余照通用的来。
pub(crate) fn describe(e: &SyncError) -> String {
  match e {
    SyncError::Network(detail) => tf("tools.account-network-error", &[detail]),
    SyncError::TooLarge => t("tools.account-response-too-large"),
    other => other.message(),
  }
}

/// 账号接口错误码的界面文案。
///
/// 词条字面量写在这里（而不是按码拼 key）：静态扫描要靠字面量认出每一条在用。
pub fn error_message(code: ErrorCode) -> String {
  match code {
    ErrorCode::Unavailable => t("tools.account-err-unavailable"),
    ErrorCode::Unauthorized => t("tools.account-err-unauthorized"),
    ErrorCode::BadCredentials => t("tools.account-err-bad-credentials"),
    ErrorCode::InvalidUsername => t("tools.account-err-invalid-username"),
    ErrorCode::InvalidPassword => t("tools.account-err-invalid-password"),
    ErrorCode::UsernameTaken => t("tools.account-err-username-taken"),
    ErrorCode::RegistrationClosed => t("tools.account-err-registration-closed"),
    ErrorCode::TooManyAttempts => t("tools.account-err-too-many-attempts"),
    ErrorCode::InvalidBackup => t("tools.account-err-invalid-backup"),
    ErrorCode::TooLarge => t("tools.account-err-too-large"),
    ErrorCode::PreconditionFailed => t("tools.account-err-precondition"),
    ErrorCode::AccountChanged => t("tools.account-err-account-changed"),
    ErrorCode::BadRequest => t("tools.account-err-bad-request"),
    ErrorCode::Internal => t("tools.account-err-internal"),
  }
}

// ───────────────────────── 登录态与账号操作 ─────────────────────────

/// 对 `/session` 的回答该怎么处置。
#[derive(Debug, PartialEq)]
enum Verdict {
  /// 采用这个登录态。
  Apply(AccountState),
  /// 问不出结果（不是"服务器说没启用"）：保留已知的登录态。
  KeepKnown,
}

fn verdict(reply: Result<(u16, String), SyncError>) -> Verdict {
  let (status, body) = match reply {
    // 网络不通、被限流、网关或服务端故障：这些都说明"这一刻没问成"，不说明"没启用账号"。
    // 把它们当成 `Unavailable`，一次 502 就会让已登录的页面掉成"服务器未启用"，
    // 还会卸载打开着的改口令对话框。
    Err(_) => return Verdict::KeepKnown,
    Ok((status, _)) if status == 429 || status >= 500 => return Verdict::KeepKnown,
    Ok(reply) => reply,
  };
  // 4xx 等：这个地址上没有账号接口（纯静态托管的 404 之类）。
  if !is_success(status) {
    return Verdict::Apply(AccountState::Unavailable);
  }
  Verdict::Apply(match serde_json::from_str::<SessionInfo>(&body) {
    Ok(info) if info.enabled => match info.user {
      Some(user) => AccountState::SignedIn(user),
      None => AccountState::SignedOut {
        registration_open: info.registration_open,
      },
    },
    // 没启用，或返回的不是账号接口的 JSON（纯静态托管时回退到了 `index.html`）。
    _ => AccountState::Unavailable,
  })
}

/// 向服务器问一次登录态并更新 [`state`]。
///
/// 问不出结果时**保留**上一次已知的登录态（只有从没问成功过才落到 `Unavailable`）：
/// 离线几秒、一次 5xx 都不该把一个已登录的页面变成「服务不可用」。
/// 等待期间登录态已被别处明确改过（刚登录 / 刚退出 / 另一次刷新先回来），这次的答案作废。
pub async fn refresh() -> AccountState {
  let epoch = EPOCH.with(Cell::get);
  let reply = call("GET", "/session", None, false, Precondition::None).await;
  let known = STATE.with(|s| s.get_untracked());
  if EPOCH.with(Cell::get) != epoch {
    return known;
  }
  match verdict(reply) {
    Verdict::Apply(next) => {
      set_state(next.clone());
      next
    }
    Verdict::KeepKnown if known == AccountState::Unknown => {
      set_state(AccountState::Unavailable);
      AccountState::Unavailable
    }
    Verdict::KeepKnown => known,
  }
}

/// 登录或注册成功后记下登录态。
fn signed_in(body: &str) -> Result<UserInfo, SyncError> {
  match serde_json::from_str::<SessionInfo>(body)
    .ok()
    .and_then(|i| i.user)
  {
    Some(user) => {
      set_state(AccountState::SignedIn(user.clone()));
      Ok(user)
    }
    None => Err(SyncError::Account(ErrorCode::Internal)),
  }
}

async fn authenticate(path: &str, username: &str, password: &str) -> Result<UserInfo, String> {
  let body = serde_json::to_string(&Credentials {
    username: username.to_owned(),
    password: password.to_owned(),
  })
  .map_err(|_| t("tools.account-err-bad-request"))?;
  let (status, text) = call("POST", path, Some(body), false, Precondition::None)
    .await
    .map_err(|e| describe(&e))?;
  if is_success(status) {
    signed_in(&text).map_err(|e| describe(&e))
  } else {
    Err(describe(&failure(status, &text)))
  }
}

/// 登录。错误已是界面文案。
pub async fn login(username: &str, password: &str) -> Result<UserInfo, String> {
  authenticate("/login", username, password).await
}

/// 注册（成功即登录）。错误已是界面文案。
pub async fn register(username: &str, password: &str) -> Result<UserInfo, String> {
  authenticate("/register", username, password).await
}

/// 退出登录。服务端会话失效、Cookie 被清；错误已是界面文案。
pub async fn logout() -> Result<(), String> {
  let (status, text) = call(
    "POST",
    "/logout",
    Some("{}".into()),
    false,
    Precondition::None,
  )
  .await
  .map_err(|e| describe(&e))?;
  if !is_success(status) {
    return Err(describe(&failure(status, &text)));
  }
  // 先按「已退出」记下（刷新失败时也不能留着已登录的假象），再以服务器说的为准：
  // 另一个标签页可能已经登录了别的账号。
  set_state(AccountState::SignedOut {
    registration_open: true,
  });
  refresh().await;
  Ok(())
}

/// 修改口令。成功后其他设备的登录失效。错误已是界面文案。
pub async fn change_password(current: &str, new: &str) -> Result<(), String> {
  let body = serde_json::to_string(&ChangePassword {
    current: current.to_owned(),
    new: new.to_owned(),
  })
  .map_err(|_| t("tools.account-err-bad-request"))?;
  let (status, text) = call("POST", "/password", Some(body), true, Precondition::None)
    .await
    .map_err(|e| describe(&e))?;
  if is_success(status) {
    Ok(())
  } else {
    Err(describe(&failure(status, &text)))
  }
}

/// 注销账号（云端备份一并删除）。错误已是界面文案。
pub async fn delete_account(password: &str) -> Result<(), String> {
  let body = serde_json::to_string(&ConfirmPassword {
    password: password.to_owned(),
  })
  .map_err(|_| t("tools.account-err-bad-request"))?;
  let (status, text) = call("POST", "/delete", Some(body), true, Precondition::None)
    .await
    .map_err(|e| describe(&e))?;
  if !is_success(status) {
    return Err(describe(&failure(status, &text)));
  }
  // 先按「已退出」记下（刷新失败时也不能留着已登录的假象），再以服务器说的为准：
  // 另一个标签页可能已经登录了别的账号。
  set_state(AccountState::SignedOut {
    registration_open: true,
  });
  refresh().await;
  Ok(())
}

// ───────────────────────── 手动同步 ─────────────────────────

/// 从云端取回备份 JSON，**不动本机数据**。错误已是带前缀的界面文案。
async fn fetch(timeout_ms: i32) -> Result<Fetched, String> {
  let fail = |detail: String| tf("tools.account-pull-failed", &[&detail]);
  let (status, body) = call_within("GET", "/backup", None, true, Precondition::None, timeout_ms)
    .await
    .map_err(|e| fail(describe(&e)))?;
  match status {
    204 => Ok(Fetched::Empty),
    200..=299 if body.trim().is_empty() => Ok(Fetched::Empty),
    200..=299 => Ok(Fetched::Data(body)),
    other => Err(fail(describe(&failure(other, &body)))),
  }
}

/// 云端内容的清单（按账号通道的范围）；内容不是 `{ key: value }` 的备份时为 `None`。
fn manifest_of_body(body: &str) -> Option<Manifest> {
  let map: BTreeMap<String, String> = serde_json::from_str(body).ok()?;
  Some(manifest_of(&map, include_images()))
}

/// 手动：从云端恢复到本机（`merge` 为真走合并导入）。
///
/// 恢复完把「两边当时都停在云端那一版」记成基线（见 `ham_web_core::delta::base_after_pull`），
/// 这样恢复之后本机与云端不一样的部分——合并出的结果、本机独有的数据——下一次自动同步会当成
/// 「本机改了」传上去，而不是被当成「两边都改了」卡住。
pub async fn pull(merge: bool) -> Result<PullResult, String> {
  let fetched = fetch(TIMEOUT_MS).await?;
  let remote = match &fetched {
    Fetched::Data(body) => manifest_of_body(body),
    Fetched::Empty => None,
  };
  let result = import_fetched(
    fetched,
    merge,
    "tools.account-pull-failed",
    "tools.account-not-a-backup",
  )
  .await?;
  let PullResult::Imported { count, fp, .. } = result else {
    return Ok(result);
  };
  // 恢复之后本机现在是什么样：读不出来（体积超限等）就不给基线，自动同步会当成从没同步过逐个 key 对比。
  let base = match (remote, crate::util::build_sync_backup(modules()).await) {
    (Some(remote), Ok(local)) => Some(base_after_pull(
      &manifest_of(&local, include_images()),
      &remote,
    )),
    _ => None,
  };
  Ok(PullResult::Imported { count, fp, base })
}

/// 准备好、还没发出的一次写入。
struct Built {
  map: BTreeMap<String, String>,
  /// 本机内容的清单：写入成功后据此建立基线。
  local: Manifest,
}

async fn build() -> Result<Built, PushError> {
  // 生成失败的文案本身就是完整的一句（如「备份超过 64 MiB」），不再套前缀。
  let map = crate::util::build_sync_backup(modules())
    .await
    .map_err(PushError::permanent)?;
  let local = manifest_of(&map, include_images());
  Ok(Built { map, local })
}

async fn send(built: Built) -> Result<Pushed, PushError> {
  let Built { map, local } = built;
  let count = map.len();
  let json = crate::util::serialize_backup(&map).map_err(PushError::permanent)?;
  // 与自定义远端同理：上传期间不要让 Rust 侧的键值表与 JSON 陪着 JS 侧那份。
  drop(map);
  let (status, body) = call("PUT", "/backup", Some(json), true, Precondition::None)
    .await
    .map_err(|e| PushError::from_account(&e))?;
  if !is_success(status) {
    return Err(PushError::from_account(&failure(status, &body)));
  }
  let receipt = serde_json::from_str::<BackupReceipt>(&body).ok();
  // 基线用服务端回执里的清单：它按服务端规范化之后的内容算，下次取回的就是这份。
  // 回执读不出来（不会发生）就不给基线，自动同步会逐个 key 对比。
  let (fp, base) = match receipt {
    Some(r) => {
      let remote = r
        .keys
        .into_iter()
        .filter(|(key, _)| ham_web_core::delta::in_scope(key, include_images()))
        .collect::<Manifest>();
      (r.info.fingerprint, Some(base_after_push(&local, &remote)))
    }
    None => (manifest_fingerprint(&local), None),
  };
  Ok(Pushed { count, fp, base })
}

/// 手动：把本机备份传到云端，一定传、**不设条件**（用户明确要覆盖云端）。
pub async fn push() -> Result<Pushed, String> {
  let built = build().await.map_err(|e| e.message)?;
  send(built).await.map_err(|e| e.message)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn session_json(enabled: bool, user: bool) -> String {
    let user = if user {
      r#"{"id":7,"username":"BG4XXX","created_at":1,"backup":null}"#
    } else {
      "null"
    };
    format!(r#"{{"enabled":{enabled},"registration_open":true,"user":{user}}}"#)
  }

  #[test]
  fn a_good_answer_is_applied() {
    match verdict(Ok((200, session_json(true, true)))) {
      Verdict::Apply(AccountState::SignedIn(u)) => assert_eq!(u.id, 7),
      other => panic!("{other:?}"),
    }
    assert_eq!(
      verdict(Ok((200, session_json(true, false)))),
      Verdict::Apply(AccountState::SignedOut {
        registration_open: true
      })
    );
    // 服务器明确说没启用。
    assert_eq!(
      verdict(Ok((200, session_json(false, false)))),
      Verdict::Apply(AccountState::Unavailable)
    );
  }

  #[test]
  fn a_page_that_is_not_the_account_api_means_unavailable() {
    // 纯静态托管：`/api/account/session` 回退到了 index.html（200）或 404。
    assert_eq!(
      verdict(Ok((200, "<html>app</html>".into()))),
      Verdict::Apply(AccountState::Unavailable)
    );
    assert_eq!(
      verdict(Ok((404, String::new()))),
      Verdict::Apply(AccountState::Unavailable)
    );
  }

  #[test]
  fn a_hiccup_never_changes_what_we_know() {
    assert_eq!(
      verdict(Err(SyncError::Network("offline".into()))),
      Verdict::KeepKnown
    );
    assert_eq!(verdict(Err(SyncError::Timeout)), Verdict::KeepKnown);
    for status in [429, 500, 502, 503, 504] {
      assert_eq!(
        verdict(Ok((status, String::new()))),
        Verdict::KeepKnown,
        "{status}"
      );
    }
  }

  #[test]
  fn an_error_code_in_the_body_beats_the_http_status() {
    let body = r#"{"error":{"code":"precondition_failed","message":"x"}}"#;
    assert!(matches!(
      failure(412, body),
      SyncError::Account(ErrorCode::PreconditionFailed)
    ));
    // 看不懂的码（比自己新的服务端加的）、不是 JSON：退回按状态码说。
    assert!(matches!(
      failure(418, r#"{"error":{"code":"from_the_future"}}"#),
      SyncError::Http(418)
    ));
    assert!(matches!(
      failure(502, "<html>bad gateway</html>"),
      SyncError::Http(502)
    ));
  }

  #[test]
  fn only_transient_account_errors_are_worth_retrying() {
    assert!(SyncError::Account(ErrorCode::Internal).retryable());
    assert!(SyncError::Account(ErrorCode::TooManyAttempts).retryable());
    // 这些重试多少次都一样，要用户处理。
    for code in [
      ErrorCode::Unauthorized,
      ErrorCode::PreconditionFailed,
      ErrorCode::AccountChanged,
      ErrorCode::TooLarge,
      ErrorCode::InvalidBackup,
      ErrorCode::Unavailable,
    ] {
      assert!(!SyncError::Account(code).retryable(), "{code:?}");
    }
    assert!(SyncError::Http(503).retryable());
    assert!(!SyncError::Http(412).retryable());
  }
}
