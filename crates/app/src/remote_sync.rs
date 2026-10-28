//! 自定义远端同步的浏览器侧：配置存取，以及「获取 / 写入」两个 `fetch`。
//!
//! 接口约定、校验规则与哪些 key 不进备份见 `ham_web_core::remote_sync`。这里只做 I/O，
//! 并把各种失败翻成界面文案。「什么时候自动获取 / 自动写入」是 `crate::auto_sync` 的事。
//!
//! 同步的内容就是「数据备份」那份 JSON，但**不含设备专属的 key**（主题、语言、接口缓存…，
//! 见 [`crate::util::build_sync_backup`]）；获取后交给 [`crate::util::import_backup`] /
//! [`crate::util::import_backup_merge`] 恢复，所以校验、合并、QSL 影像处理都和手动导入完全一致。
//!
//! 写出的范围永远是完整的（日志 + 其他数据，QSL 影像按配置可去掉），不跟「导出备份」的模块勾选走：
//! 远端是一份整体替换的快照，漏掉某个模块就等于把它从远端抹掉，基线也就没法再用来判断「有没有变」。

use std::cell::RefCell;
use std::rc::Rc;

use ham_web_core::backup_merge::backup_size_is_supported;
use ham_web_core::remote_sync::{
  Fingerprint, HeaderError, PullOutcome, RemoteSyncConfig, STORAGE_KEY, UrlError, check_url,
  classify_pull, fingerprint, fingerprint_json, is_retryable_status, parse_headers,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestCache, RequestInit, Response};

use crate::data::AbortTimeout;
use crate::i18n::{t, tf};
use crate::util::{BackupModules, js_error_message, storage, window};

/// 单次请求的总超时（含读完响应体）。备份可达几十 MB，比一般接口宽松得多。
const TIMEOUT_MS: i32 = 60_000;
/// 启动时自动获取的超时：这一步不阻塞界面，但自动写入要等它落定，不能让一个挂住的服务拖太久。
pub const AUTO_FETCH_TIMEOUT_MS: i32 = 15_000;

thread_local! {
  /// 配置缓存。自动同步的写入钩子在**每一次**存储写入的路径上，不能每次都去解析 JSON。
  static CONFIG: RefCell<Option<Rc<RemoteSyncConfig>>> = const { RefCell::new(None) };
}

/// 当前配置（共享，不克隆内容）。
///
/// 返回 `Rc` 而不是在 `RefCell` 借用里调回调：回调里再读写配置（保存配置 → 触发存储钩子 →
/// 钩子又来读配置）时，持着借用会 panic。
fn shared() -> Rc<RemoteSyncConfig> {
  if let Some(cfg) = CONFIG.with(|c| c.borrow().clone()) {
    return cfg;
  }
  let loaded = Rc::new(storage::get_json(STORAGE_KEY).unwrap_or_default());
  CONFIG.with(|c| *c.borrow_mut() = Some(Rc::clone(&loaded)));
  loaded
}

/// 在当前配置上执行 `f`（不克隆；`f` 里可以安全地再读写配置）。
pub fn with_config<R>(f: impl FnOnce(&RemoteSyncConfig) -> R) -> R {
  f(&shared())
}

/// 当前配置；没有或读不出来就是全空（功能关闭）。
pub fn config() -> RemoteSyncConfig {
  (*shared()).clone()
}

/// 保存配置。
pub fn save_config(cfg: &RemoteSyncConfig) {
  CONFIG.with(|c| *c.borrow_mut() = Some(Rc::new(cfg.clone())));
  storage::set_json(STORAGE_KEY, cfg);
}

/// 这份配置要同步哪些模块。
fn modules(cfg: &RemoteSyncConfig) -> BackupModules {
  BackupModules::for_sync(!cfg.exclude_qsl_images)
}

/// 一次远端请求失败的原因。
enum SyncError {
  Url(UrlError),
  Headers(HeaderError),
  Network(String),
  Timeout,
  Http(u16),
  TooLarge,
}

impl SyncError {
  /// 面向用户的说明。
  fn message(&self) -> String {
    match self {
      Self::Url(e) => url_message(*e),
      Self::Headers(e) => headers_message(*e),
      Self::Network(m) => tf("tools.remote-network-error", &[m]),
      Self::Timeout => t("tools.remote-timeout"),
      Self::Http(status) => tf("tools.remote-http-error", &[&status.to_string()]),
      Self::TooLarge => t("tools.remote-response-too-large"),
    }
  }

  /// 过一会儿再试有没有可能成功：网络 / 超时 / 服务端 5xx 是；地址、请求头、令牌、体积不对不是。
  fn retryable(&self) -> bool {
    match self {
      Self::Network(_) | Self::Timeout => true,
      Self::Http(status) => is_retryable_status(*status),
      Self::Url(_) | Self::Headers(_) | Self::TooLarge => false,
    }
  }
}

fn url_message(e: UrlError) -> String {
  match e {
    UrlError::Empty | UrlError::Invalid => t("tools.remote-url-invalid"),
    UrlError::Credentials => t("tools.remote-url-credentials"),
    UrlError::MixedContent => t("tools.remote-url-mixed-content"),
  }
}

fn headers_message(e: HeaderError) -> String {
  tf("tools.remote-header-invalid", &[&e.line.to_string()])
}

fn page_is_https() -> bool {
  window().location().protocol().is_ok_and(|p| p == "https:")
}

/// 地址的就地校验文案：没填不算错（功能本来就可以只配一个接口），填了但不合格才有。
pub fn url_error(url: &str) -> Option<String> {
  match check_url(url, page_is_https()) {
    Err(UrlError::Empty) | Ok(()) => None,
    Err(e) => Some(url_message(e)),
  }
}

/// 请求头的就地校验文案；空文本合法。
pub fn headers_error(text: &str) -> Option<String> {
  parse_headers(text).err().map(headers_message)
}

/// 解析好的请求头：`(名称, 值)`。
type HeaderList = Vec<(String, String)>;

/// 发请求前的统一校验：地址合格、请求头可解析。
fn prepare<'a>(url: &'a str, headers: &str) -> Result<(&'a str, HeaderList), SyncError> {
  check_url(url, page_is_https()).map_err(SyncError::Url)?;
  let headers = parse_headers(headers).map_err(SyncError::Headers)?;
  Ok((url.trim(), headers))
}

/// `AbortController` 因超时中止时，`fetch` 拒绝的错误名是 `AbortError`。
fn classify_js_error(e: &JsValue) -> SyncError {
  let aborted = js_sys::Reflect::get(e, &"name".into())
    .ok()
    .and_then(|n| n.as_string())
    .is_some_and(|n| n == "AbortError");
  if aborted {
    SyncError::Timeout
  } else {
    SyncError::Network(js_error_message(e))
  }
}

/// 发一次请求，返回 `(状态码, 响应体文本)`。
///
/// 超时覆盖到响应体读完为止：`AbortTimeout` 守卫活到函数返回。
/// 带请求体时才加 `Content-Type: application/json`：GET 上多一个非安全请求头就会无端多一次预检。
async fn request(
  method: &str,
  url: &str,
  headers: &[(String, String)],
  body: Option<&str>,
  timeout_ms: i32,
) -> Result<(u16, String), SyncError> {
  let net = |e: JsValue| SyncError::Network(js_error_message(&e));
  let header_map = Headers::new().map_err(net)?;
  if body.is_some() {
    header_map
      .set("Content-Type", "application/json")
      .map_err(net)?;
  }
  // 用户的头排在后面：想改 Content-Type（如某些网盘要 `text/plain`）可以覆盖。
  for (name, value) in headers {
    header_map.set(name, value).map_err(net)?;
  }

  let timeout = AbortTimeout::start(timeout_ms).map_err(net)?;
  let init = RequestInit::new();
  init.set_method(method);
  init.set_headers(&header_map);
  // 同步读的必须是远端当下的内容，不能被 HTTP 缓存顶替。
  init.set_cache(RequestCache::NoStore);
  init.set_signal(Some(&timeout.signal()));
  if let Some(body) = body {
    init.set_body(&JsValue::from_str(body));
  }

  let request = Request::new_with_str_and_init(url, &init).map_err(net)?;
  let resp: Response = JsFuture::from(window().fetch_with_request(&request))
    .await
    .map_err(|e| classify_js_error(&e))?
    .unchecked_into();

  // 先看声明的长度：明显超限的不必读进内存。
  if let Ok(Some(len)) = resp.headers().get("content-length")
    && len
      .trim()
      .parse::<usize>()
      .is_ok_and(|n| !backup_size_is_supported(n))
  {
    return Err(SyncError::TooLarge);
  }
  let text = JsFuture::from(resp.text().map_err(net)?)
    .await
    .map_err(|e| classify_js_error(&e))?
    .as_string()
    .unwrap_or_default();
  if !backup_size_is_supported(text.len()) {
    return Err(SyncError::TooLarge);
  }
  Ok((resp.status(), text))
}

/// 取回的远端内容。
pub enum Fetched {
  /// 远端还没有数据（`404` / `204` / 空内容）。
  Empty,
  /// 响应体原文（还没校验是不是备份）。
  Data(String),
}

/// 从获取接口取回备份 JSON，**不动本机数据**。错误已是带前缀的界面文案。
pub async fn fetch(cfg: &RemoteSyncConfig, timeout_ms: i32) -> Result<Fetched, String> {
  let fail = |detail: String| tf("tools.remote-pull-failed", &[&detail]);
  let (url, headers) = prepare(&cfg.get_url, &cfg.get_headers).map_err(|e| fail(e.message()))?;
  let (status, body) = request("GET", url, &headers, None, timeout_ms)
    .await
    .map_err(|e| fail(e.message()))?;
  match classify_pull(status, &body) {
    PullOutcome::Empty => Ok(Fetched::Empty),
    PullOutcome::Failed(code) => Err(fail(SyncError::Http(code).message())),
    PullOutcome::Data => Ok(Fetched::Data(body)),
  }
}

/// 远端内容的指纹；不是 `{ key: value }` 形态的备份时给出用户能看懂的错误。
pub fn fingerprint_of(body: &str) -> Result<Fingerprint, String> {
  fingerprint_json(body).ok_or_else(|| {
    tf(
      "tools.remote-pull-failed",
      &[&t("tools.remote-not-a-backup")],
    )
  })
}

/// 手动拉取的结果。
pub enum PullResult {
  /// 远端还没有数据（`404` / `204` / 空内容）。
  Empty,
  /// 已恢复。
  Imported {
    /// 恢复的条目数。
    count: usize,
    /// 远端内容的指纹，交给自动同步当基线。
    fp: Fingerprint,
  },
}

/// 手动：从获取接口拉取备份 JSON 并恢复到本机。
///
/// `merge` 为真时走「合并导入」（并集 / 累加），否则整体覆盖。错误已是带前缀的界面文案。
pub async fn pull(cfg: &RemoteSyncConfig, merge: bool) -> Result<PullResult, String> {
  let Fetched::Data(body) = fetch(cfg, TIMEOUT_MS).await? else {
    return Ok(PullResult::Empty);
  };
  let fp = fingerprint_of(&body)?;
  let result = if merge {
    crate::util::import_backup_merge(&body).await
  } else {
    crate::util::import_backup(&body).await
  };
  result
    .map(|count| PullResult::Imported { count, fp })
    .map_err(|e| tf("tools.remote-pull-failed", &[&e]))
}

/// 一次成功写出的结果。
pub struct Pushed {
  /// 写出的条目数。
  pub count: usize,
  /// 写出内容的指纹，交给自动同步当基线。
  pub fp: Fingerprint,
}

/// 写入失败：界面文案，以及自动写入该不该继续重试。
pub struct PushError {
  /// 已经是带前缀的界面文案。
  pub message: String,
  /// 过一会儿再试有没有可能成功（见 [`SyncError::retryable`]）。
  ///
  /// 为否时自动写入应当暂停并说明原因：令牌不对、地址写错、数据超限这类问题，重试多少次都一样，
  /// 而每次重试都要把整个本地库读一遍再上传。
  pub retryable: bool,
}

impl PushError {
  /// 重试无益的失败（本机数据或配置本身的问题）。
  fn permanent(message: String) -> Self {
    Self {
      message,
      retryable: false,
    }
  }

  fn from_sync(e: &SyncError) -> Self {
    let fail = tf("tools.remote-push-failed", &[&e.message()]);
    Self {
      message: fail,
      retryable: e.retryable(),
    }
  }
}

/// 准备好、还没发出的一次写入。
struct Built {
  url: String,
  headers: HeaderList,
  map: std::collections::BTreeMap<String, String>,
  fp: Fingerprint,
}

/// 校验接口配置并生成要写出的内容。先校验再生成：配置有问题时不必白白读一遍整个本地库。
async fn build(cfg: &RemoteSyncConfig) -> Result<Built, PushError> {
  let (url, headers) =
    prepare(&cfg.put_url, &cfg.put_headers).map_err(|e| PushError::from_sync(&e))?;
  // 生成失败的文案本身就是完整的一句（如「备份超过 64 MiB」），不再套前缀。
  let map = crate::util::build_sync_backup(modules(cfg))
    .await
    .map_err(PushError::permanent)?;
  let fp = fingerprint(&map);
  Ok(Built {
    url: url.to_owned(),
    headers,
    map,
    fp,
  })
}

async fn send(cfg: &RemoteSyncConfig, built: Built) -> Result<Pushed, PushError> {
  let json = crate::util::serialize_backup(&built.map).map_err(PushError::permanent)?;
  let (status, _) = request(
    cfg.method.as_str(),
    &built.url,
    &built.headers,
    Some(&json),
    TIMEOUT_MS,
  )
  .await
  .map_err(|e| PushError::from_sync(&e))?;
  if (200..300).contains(&status) {
    Ok(Pushed {
      count: built.map.len(),
      fp: built.fp,
    })
  } else {
    Err(PushError::from_sync(&SyncError::Http(status)))
  }
}

/// 手动：把本机备份发给写入接口，一定发。错误已是带前缀的界面文案。
pub async fn push(cfg: &RemoteSyncConfig) -> Result<Pushed, String> {
  let built = build(cfg).await.map_err(|e| e.message)?;
  send(cfg, built).await.map_err(|e| e.message)
}

/// 自动：本机内容的指纹与 `base` 相同就**不发请求**，返回 `Ok(None)`。
///
/// 很多存储写入只是写了同样的值（或只动了设备专属的 key），靠指纹跳过这些没有实际变化的触发。
pub async fn push_if_changed(
  cfg: &RemoteSyncConfig,
  base: Option<Fingerprint>,
) -> Result<Option<Pushed>, PushError> {
  let built = build(cfg).await?;
  if base == Some(built.fp) {
    return Ok(None);
  }
  send(cfg, built).await.map(Some)
}
