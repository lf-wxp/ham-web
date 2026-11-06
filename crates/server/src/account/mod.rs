//! 在线账号：注册 / 登录 / 云端备份（`/api/account/*`），数据存在 SQLite。
//!
//! 它与前端的「自定义远端同步」是**并列的两条同步通道**，交换同一份备份 JSON；
//! 区别只在去处——这里的去处是本服务器 SQLite 里的多张表（见 `relational`）。两者怎么共存、基线与冲突怎么判断，
//! 见 `docs/domain/account-sync.md`；接口契约见 `docs/reference/api.md`。
//!
//! # 环境变量
//!
//! | 变量 | 默认 | 说明 |
//! | --- | --- | --- |
//! | `ACCOUNT_DB` | 未设置 = 不启用 | SQLite 文件路径；Docker 镜像里是 `/app/data/accounts.db`，要挂数据卷 |
//! | `ACCOUNT_REGISTRATION` | `open` | `open` / `closed`；`closed` 关闭注册（已有账号照常登录） |
//! | `ACCOUNT_COOKIE_SECURE` | `auto` | `auto`：经 `X-Forwarded-Proto: https` 才加 `Secure`；`1` / `0` 强制 |
//! | `ACCOUNT_SESSION_DAYS` | `30` | 会话有效期（滑动续期），1–3650 |
//! | `ACCOUNT_MAX_USERS` | `1000` | 账号总数上限，满了就不再收新账号；`0` = 不限 |
//!
//! 变量**写错值会让启动失败**，而不是悄悄回退到默认：`ACCOUNT_REGISTRATION=close`（少个 `d`）
//! 若被当成「开放」，运维以为关了、实际全网可注册，比起动不起来危害大得多。
//!
//! 没设 `ACCOUNT_DB` 时整组接口返回 `503 unavailable`，前端据此隐藏登录入口：
//! 既有部署升级后行为不变，也不会在没人知情的地方悄悄多出一个数据库文件。

mod error;
mod handlers;
mod password;
mod relational;
mod session;
mod store;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use ham_web_core::account::{API_BASE, SYNC_MANIFEST_PATH, SYNC_PULL_PATH, SYNC_PUSH_PATH};
use ham_web_core::backup_merge::MAX_BACKUP_BYTES;
use tokio::sync::Semaphore;

use crate::rate_limit::RateLimiter;

use self::store::Store;

/// 同时进行的 Argon2 运算数：每个占 ~19 MiB 内存，并发不设限就能被一批登录请求撑爆进程。
const HASH_SLOTS: usize = 4;
/// 备份请求体上限：备份本身的上限再留一点 HTTP 层的余量（JSON 转义不会超出它，这里只是防御）。
const BODY_LIMIT: usize = MAX_BACKUP_BYTES + 64 * 1024;
/// 同时处理的备份上传数：每个都要把请求体（至多 64 MiB）读进内存再解析，不设限就能被几个并发请求撑爆。
const UPLOAD_SLOTS: usize = 4;
/// 按 key 取值的请求体上限：几百个 key 名，远用不到这个数。
pub(crate) const PULL_BODY_LIMIT: usize = 1024 * 1024;
/// 登录 / 注册 / 改口令 / 注销这类小 JSON 请求体的上限：账号名 ≤ 32、口令 ≤ 128 字节，
/// 转义放大后也远不到这个数；没有它就是 axum 默认的 2 MiB，匿名请求能拿来占内存。
const AUTH_BODY_LIMIT: usize = 16 * 1024;
/// 每个 IP 在窗口内的登录 / 注册尝试数（成败都算：这一层防的是单个来源刷接口）。
const IP_ATTEMPTS: usize = 20;
/// 每个（账号名，来源 IP）在窗口内允许的口令**失败**次数。只数失败：口令对了不耗额度。
const USER_ATTEMPTS: usize = 10;
/// 每个账号名（不分来源）在窗口内允许的口令失败总数：挡「换着 IP 轮流试同一个账号」。
/// 刻意高于单来源额度——别人只要知道账号名就能制造失败，额度太低等于让他随手锁死本人；
/// 单来源的 10 次先拦住单个攻击者，这一层只在分布式猜口令时才会被碰到。
const NAME_FAILURES: usize = 100;
/// 限流窗口。
const ATTEMPT_WINDOW: Duration = Duration::from_secs(10 * 60);
/// 账号总数的默认上限：个人站点绰绰有余，又不至于让开放注册变成无限占盘。
const DEFAULT_MAX_USERS: usize = 1000;

/// `Secure` 属性的取法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieSecure {
  /// 请求经 HTTPS 反代（`X-Forwarded-Proto: https`）时才加。
  Auto,
  /// 总是加（直接对外提供 HTTPS，或反代不转发协议头）。
  Always,
  /// 从不加（纯 HTTP 内网部署）。
  Never,
}

/// 在线账号的配置。
#[derive(Debug, Clone)]
pub struct AccountConfig {
  /// SQLite 文件路径；`None` = 不启用。
  pub db: Option<PathBuf>,
  /// 是否开放注册。
  pub registration_open: bool,
  /// `Secure` 属性。
  pub cookie_secure: CookieSecure,
  /// 会话有效期（秒）。
  pub session_ttl_secs: i64,
  /// 账号总数上限；`None` = 不限。
  pub max_users: Option<usize>,
}

impl Default for AccountConfig {
  fn default() -> Self {
    Self {
      db: None,
      registration_open: true,
      cookie_secure: CookieSecure::Auto,
      session_ttl_secs: 30 * 24 * 3600,
      max_users: Some(DEFAULT_MAX_USERS),
    }
  }
}

/// `ACCOUNT_REGISTRATION` 的值（不分大小写）；不认识返回 `None`。
fn parse_registration(v: &str) -> Option<bool> {
  match v.to_ascii_lowercase().as_str() {
    "open" | "true" | "1" | "yes" | "on" => Some(true),
    "closed" | "false" | "0" | "no" | "off" => Some(false),
    _ => None,
  }
}

/// `ACCOUNT_COOKIE_SECURE` 的值（不分大小写）；不认识返回 `None`。
fn parse_cookie_secure(v: &str) -> Option<CookieSecure> {
  match v.to_ascii_lowercase().as_str() {
    "auto" => Some(CookieSecure::Auto),
    "1" | "true" | "yes" | "on" => Some(CookieSecure::Always),
    "0" | "false" | "no" | "off" => Some(CookieSecure::Never),
    _ => None,
  }
}

impl AccountConfig {
  /// 从环境变量读取。
  ///
  /// # Errors
  ///
  /// 有变量设了不认识的值（见模块文档：写错就启动失败，不悄悄回退）。
  pub fn from_env() -> anyhow::Result<Self> {
    Self::from_lookup(|k| std::env::var(k).ok())
  }

  /// [`from_env`](Self::from_env) 的可测版本：`get` 按变量名取值。
  fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
    let env = |k: &str| {
      get(k)
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
    };
    let defaults = Self::default();

    let registration_open = match env("ACCOUNT_REGISTRATION") {
      None => defaults.registration_open,
      Some(v) => parse_registration(&v).with_context(|| {
        format!("环境变量 ACCOUNT_REGISTRATION 的值 {v:?} 不认识：请用 open 或 closed")
      })?,
    };
    let cookie_secure = match env("ACCOUNT_COOKIE_SECURE") {
      None => defaults.cookie_secure,
      Some(v) => parse_cookie_secure(&v).with_context(|| {
        format!("环境变量 ACCOUNT_COOKIE_SECURE 的值 {v:?} 不认识：请用 auto、1 或 0")
      })?,
    };
    let session_ttl_secs = match env("ACCOUNT_SESSION_DAYS") {
      None => defaults.session_ttl_secs,
      Some(v) => match v.parse::<i64>() {
        Ok(d) if (1..=3650).contains(&d) => d * 24 * 3600,
        _ => bail!("环境变量 ACCOUNT_SESSION_DAYS 的值 {v:?} 不合法：请用 1–3650 的整数（天）"),
      },
    };
    let max_users = match env("ACCOUNT_MAX_USERS") {
      None => defaults.max_users,
      Some(v) => match v.parse::<usize>() {
        Ok(0) => None,
        Ok(n) => Some(n),
        Err(_) => bail!("环境变量 ACCOUNT_MAX_USERS 的值 {v:?} 不合法：请用非负整数（0 = 不限）"),
      },
    };
    Ok(Self {
      db: env("ACCOUNT_DB").map(PathBuf::from),
      registration_open,
      cookie_secure,
      session_ttl_secs,
      max_users,
    })
  }
}

struct Inner {
  store: Arc<Store>,
  cfg: AccountConfig,
  /// 限制并发的 Argon2 运算。
  hash_slots: Semaphore,
  /// 限制同时处理的备份上传。
  upload_slots: Semaphore,
}

/// 在线账号服务；没启用时 `inner` 为空，所有接口回 `unavailable`。
#[derive(Clone)]
pub struct AccountService {
  inner: Option<Arc<Inner>>,
  /// 按 IP：登录 / 注册的尝试（成败都算）。
  ip_attempts: RateLimiter,
  /// 按（账号名，IP）：口令失败次数。
  user_attempts: RateLimiter,
  /// 按账号名：口令失败总数（不分来源）。
  name_failures: RateLimiter,
}

impl AccountService {
  fn build(inner: Option<Inner>) -> Self {
    Self {
      inner: inner.map(Arc::new),
      ip_attempts: RateLimiter::new(IP_ATTEMPTS, ATTEMPT_WINDOW),
      user_attempts: RateLimiter::new(USER_ATTEMPTS, ATTEMPT_WINDOW),
      name_failures: RateLimiter::new(NAME_FAILURES, ATTEMPT_WINDOW),
    }
  }

  /// 没启用在线账号。
  pub fn disabled() -> Self {
    Self::build(None)
  }

  /// 按配置创建；配了 `db` 就打开（必要时创建、迁移）数据库。
  ///
  /// # Errors
  ///
  /// 数据库打不开：启动时就该失败，而不是等第一个用户注册时才发现。
  pub fn new(cfg: AccountConfig) -> anyhow::Result<Self> {
    let Some(path) = cfg.db.clone() else {
      return Ok(Self::disabled());
    };
    let store = Store::open(&path).context("初始化在线账号数据库失败")?;
    tracing::info!(
      "在线账号已启用：数据库 {}，{}注册",
      path.display(),
      if cfg.registration_open {
        "开放"
      } else {
        "关闭"
      }
    );
    Ok(Self::build(Some(Inner {
      store: Arc::new(store),
      cfg,
      hash_slots: Semaphore::new(HASH_SLOTS),
      upload_slots: Semaphore::new(UPLOAD_SLOTS),
    })))
  }

  /// 内存数据库（测试用）。
  #[cfg(test)]
  pub fn in_memory_with(cfg: AccountConfig) -> Self {
    Self::build(Some(Inner {
      store: Arc::new(Store::open_in_memory().expect("in-memory db")),
      cfg,
      hash_slots: Semaphore::new(HASH_SLOTS),
      upload_slots: Semaphore::new(UPLOAD_SLOTS),
    }))
  }

  #[cfg(test)]
  pub fn in_memory(registration_open: bool) -> Self {
    Self::in_memory_with(AccountConfig {
      registration_open,
      ..AccountConfig::default()
    })
  }

  /// 每小时清一次过期会话。
  pub fn spawn_maintenance(&self) {
    let Some(inner) = self.inner.clone() else {
      return;
    };
    tokio::spawn(async move {
      let mut tick = tokio::time::interval(Duration::from_secs(3600));
      loop {
        tick.tick().await;
        let store = inner.store.clone();
        let now = handlers::now();
        match tokio::task::spawn_blocking(move || store.purge_expired_sessions(now)).await {
          Ok(Ok(n)) if n > 0 => tracing::info!("清理了 {n} 个过期会话"),
          Ok(Err(e)) => tracing::warn!("清理过期会话失败：{e:?}"),
          _ => {}
        }
      }
    });
  }

  /// 挂到主路由上的 `/api/account/*`。
  pub fn router(&self) -> Router {
    // 登录 / 注册要过 IP 限流；其余接口已经要求有效会话，不额外限。
    // 这里全用 `route_layer`：只包住账号自己的路由，不碰合并进主路由时的 fallback。
    let guarded = Router::new()
      .route(&format!("{API_BASE}/login"), post(handlers::login))
      .route(&format!("{API_BASE}/register"), post(handlers::register))
      .route_layer(middleware::from_fn_with_state(
        self.clone(),
        handlers::limit_attempts,
      ))
      .route_layer(DefaultBodyLimit::max(AUTH_BODY_LIMIT));
    Router::new()
      .route(&format!("{API_BASE}/session"), get(handlers::session))
      .route(&format!("{API_BASE}/logout"), post(handlers::logout))
      .route(
        &format!("{API_BASE}/password"),
        post(handlers::change_password).layer(DefaultBodyLimit::max(AUTH_BODY_LIMIT)),
      )
      .route(
        &format!("{API_BASE}/delete"),
        post(handlers::delete_account).layer(DefaultBodyLimit::max(AUTH_BODY_LIMIT)),
      )
      // 备份的请求体由 `put_backup` 自己在鉴权之后、拿到上传名额之后才读（并带上限），
      // 所以这里不挂 `DefaultBodyLimit`：未登录的请求读不到一个字节。
      .route(
        &format!("{API_BASE}/backup"),
        get(handlers::get_backup)
          .put(handlers::put_backup)
          .delete(handlers::delete_backup),
      )
      // 按 key 的增量同步：清单是读目录，取值请求体很小；上传和整份上传一样由处理函数自己鉴权后再读体。
      .route(
        &format!("{API_BASE}{SYNC_MANIFEST_PATH}"),
        get(handlers::sync_manifest),
      )
      .route(
        &format!("{API_BASE}{SYNC_PULL_PATH}"),
        // 与上传一样由处理函数自己鉴权后再读体（带 `PULL_BODY_LIMIT` 上限），这里不挂 `DefaultBodyLimit`。
        post(handlers::sync_pull),
      )
      .route(
        &format!("{API_BASE}{SYNC_PUSH_PATH}"),
        post(handlers::sync_push),
      )
      .merge(guarded)
      .route_layer(middleware::from_fn(handlers::tag_client_ip))
      .with_state(self.clone())
  }
}

#[cfg(test)]
mod tests;
