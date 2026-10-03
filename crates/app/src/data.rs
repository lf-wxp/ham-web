//! 题库数据加载：配置文件、题库 JSON、版本可用性状态（带内存缓存）。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use ham_web_core::glossary::{GLOSSARY_FILES, Glossary};
use ham_web_core::{Bank, BankConfig, QuestionItem, QuestionSearchEntry, QuestionVersion};
use leptos::prelude::{ArcRwSignal, Track, Update};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestCache, RequestInit, Response};

use crate::i18n::tf;
use crate::util::{js_error_message, now_ms, window};

/// 数据加载错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
  /// 网络或 HTTP 状态错误。
  Network(String),
  /// 数据格式错误。
  Validation(String),
}

impl fmt::Display for AppError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Network(m) | Self::Validation(m) => f.write_str(m),
    }
  }
}

impl std::error::Error for AppError {}

/// 共享题库数据。
pub type Questions = Arc<Vec<QuestionItem>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheMode {
  /// 优先使用 HTTP 缓存。
  ForceCache,
  /// 强制向服务器验证，并附带时间戳参数。
  NoCache,
  /// 浏览器默认缓存策略（遵循服务端缓存头，可被 Service Worker 预缓存命中）。
  Default,
}

async fn fetch_text(url: &str, mode: CacheMode) -> Result<String, AppError> {
  let init = RequestInit::new();
  let final_url = match mode {
    CacheMode::Default => url.to_owned(),
    CacheMode::ForceCache => {
      init.set_cache(RequestCache::ForceCache);
      url.to_owned()
    }
    CacheMode::NoCache => {
      init.set_cache(RequestCache::NoCache);
      let headers = web_sys::Headers::new().map_err(|e| AppError::Network(js_error_message(&e)))?;
      let _ = headers.set("Cache-Control", "no-cache, no-store, must-revalidate");
      let _ = headers.set("Pragma", "no-cache");
      init.set_headers(&headers);
      let sep = if url.contains('?') { '&' } else { '?' };
      format!("{url}{sep}t={}", now_ms())
    }
  };
  let request = Request::new_with_str_and_init(&final_url, &init)
    .map_err(|e| AppError::Network(js_error_message(&e)))?;
  let resp: Response = JsFuture::from(window().fetch_with_request(&request))
    .await
    .map_err(|e| AppError::Network(js_error_message(&e)))?
    .unchecked_into();
  if !resp.ok() {
    return Err(AppError::Network(format!(
      "HTTP {} {}",
      resp.status(),
      resp.status_text()
    )));
  }
  let text = JsFuture::from(
    resp
      .text()
      .map_err(|e| AppError::Network(js_error_message(&e)))?,
  )
  .await
  .map_err(|e| AppError::Network(js_error_message(&e)))?;
  Ok(text.as_string().unwrap_or_default())
}

/// API 离线缓存：实时数据页在离线时回退到「最后一次成功拉取」的数据，而非硬降级为
/// 「数据暂不可用」。缓存以响应原文（text）存储、以 URL 为 key，避免与各接口的解析类型耦合。
const API_CACHE_PREFIX: &str = "api-cache:";
/// 单条缓存最大 UTF-16 单元数（约 512 KB），超过不缓存，避免挤占 localStorage 配额。
const API_CACHE_MAX_UNITS: usize = 512 * 1024;
/// 缓存回退的最大有效期（7 天，毫秒），超过则认为过于陈旧、不再回退。
const API_CACHE_MAX_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// 一条 API 缓存：`{ ts: 写入时间戳(ms), text: 响应原文 }`。
#[derive(serde::Serialize, serde::Deserialize)]
struct ApiCacheEntry {
  ts: i64,
  text: String,
}

fn api_cache_key(url: &str) -> String {
  format!("{API_CACHE_PREFIX}{url}")
}

/// 成功拉取后把响应原文写入缓存（静默：配额满时不打扰用户）。
fn api_cache_put(url: &str, text: &str) {
  if text.is_empty() || text.encode_utf16().count() > API_CACHE_MAX_UNITS {
    return;
  }
  let entry = ApiCacheEntry {
    ts: now_ms(),
    text: text.to_owned(),
  };
  crate::util::storage::set_json_silent(&api_cache_key(url), &entry);
}

/// 读取仍处于有效期内的缓存响应原文。
fn api_cache_get(url: &str) -> Option<String> {
  let entry = crate::util::storage::get_json::<ApiCacheEntry>(&api_cache_key(url))?;
  if now_ms() - entry.ts > API_CACHE_MAX_AGE_MS {
    return None;
  }
  Some(entry.text)
}

fn parse_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, AppError> {
  serde_json::from_str(text).map_err(|e| AppError::Validation(format!("Invalid JSON: {e}")))
}

/// 拉取外部 JSON（带 5 秒超时）；成功时写离线缓存，失败时回退到上次缓存。
pub async fn fetch_external_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, AppError> {
  match fetch_text_external(url, 5000).await {
    Ok(text) => {
      api_cache_put(url, &text);
      parse_json(&text)
    }
    Err(net_err) => {
      // 网络失败：回退到有效期内的缓存；缓存缺失或格式不兼容时维持原始错误。
      if let Some(text) = api_cache_get(url)
        && let Ok(data) = parse_json(&text)
      {
        return Ok(data);
      }
      Err(net_err)
    }
  }
}

/// 拉取二进制资源（紧凑编码的几何数据，如 `dxcc-entities.bin`）。
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, AppError> {
  let init = RequestInit::new();
  let request = Request::new_with_str_and_init(url, &init)
    .map_err(|e| AppError::Network(js_error_message(&e)))?;
  let resp: Response = JsFuture::from(window().fetch_with_request(&request))
    .await
    .map_err(|e| AppError::Network(js_error_message(&e)))?
    .unchecked_into();
  if !resp.ok() {
    return Err(AppError::Network(format!("HTTP {}", resp.status())));
  }
  let buf = JsFuture::from(
    resp
      .array_buffer()
      .map_err(|e| AppError::Network(js_error_message(&e)))?,
  )
  .await
  .map_err(|e| AppError::Network(js_error_message(&e)))?;
  Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// 持有 `setTimeout` 句柄，析构时清除，避免超时定时器在请求提前结束后空转。
struct ClearTimeout(i32);

impl Drop for ClearTimeout {
  fn drop(&mut self) {
    if self.0 != 0 {
      window().clear_timeout_with_handle(self.0);
    }
  }
}

/// 拉取外部文本，`timeout_ms` 毫秒超时（通过 `AbortController` 中断）。
async fn fetch_text_external(url: &str, timeout_ms: i32) -> Result<String, AppError> {
  use wasm_bindgen::JsCast;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::{AbortController, Request, RequestInit, Response};

  let init = RequestInit::new();
  let controller = AbortController::new().map_err(|e| AppError::Network(js_error_message(&e)))?;
  let signal = controller.signal();
  init.set_signal(Some(&signal));

  // 超时后中断请求
  let c = controller.clone();
  let timer = wasm_bindgen::closure::Closure::once_into_js(move || {
    c.abort();
  });
  let timer_id = window()
    .set_timeout_with_callback_and_timeout_and_arguments_0(
      timer.as_ref().unchecked_ref(),
      timeout_ms,
    )
    .unwrap_or(0);
  let _clear = ClearTimeout(timer_id);

  let request = Request::new_with_str_and_init(url, &init)
    .map_err(|e| AppError::Network(js_error_message(&e)))?;
  let resp: Response = JsFuture::from(window().fetch_with_request(&request))
    .await
    .map_err(|e| AppError::Network(js_error_message(&e)))?
    .unchecked_into();
  if !resp.ok() {
    return Err(AppError::Network(format!("HTTP {}", resp.status())));
  }
  let text = JsFuture::from(
    resp
      .text()
      .map_err(|e| AppError::Network(js_error_message(&e)))?,
  )
  .await
  .map_err(|e| AppError::Network(js_error_message(&e)))?;
  Ok(text.as_string().unwrap_or_default())
}

/// 版本可用性状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionStatus {
  pub version_id: String,
  pub is_available: bool,
  pub available_banks: Vec<Bank>,
  pub last_checked: i64,
  pub error: Option<String>,
}

/// 可用版本缓存 1 分钟。
const CACHE_DURATION: i64 = 60 * 1000;
/// 不可用版本缓存 10 秒。
const SHORT_CACHE_DURATION: i64 = 10 * 1000;

#[derive(Default)]
struct Cache {
  config: Option<Rc<BankConfig>>,
  questions: HashMap<String, Questions>,
  status: HashMap<String, VersionStatus>,
}

thread_local! {
  static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// 加载题库配置；`force` 时绕过缓存。配置版本号变化会清空题库与状态缓存。
pub async fn load_config(force: bool) -> Result<Rc<BankConfig>, AppError> {
  if !force && let Some(cfg) = CACHE.with_borrow(|c| c.config.clone()) {
    return Ok(cfg);
  }
  let text = fetch_text("/questions/config.json", CacheMode::NoCache)
    .await
    .map_err(|e| AppError::Network(format!("Failed to load question bank config: {e}")))?;
  let cfg: BankConfig = serde_json::from_str(&text)
    .map_err(|e| AppError::Validation(format!("Invalid question bank config: {e}")))?;
  let cfg = Rc::new(cfg);
  CACHE.with_borrow_mut(|c| {
    if c
      .config
      .as_ref()
      .is_some_and(|old| old.version != cfg.version)
    {
      c.questions.clear();
      c.status.clear();
    }
    c.config = Some(cfg.clone());
  });
  Ok(cfg)
}

/// 全部版本（按更新时间倒序）。
pub async fn all_versions(force: bool) -> Result<Vec<QuestionVersion>, AppError> {
  Ok(load_config(force).await?.sorted_versions())
}

/// 解析题库 JSON 地址；未指定或版本不存在时回退到 `/questions/{bank}.json`。
pub async fn resolve_url(version: Option<&str>, bank: Bank) -> String {
  let Some(version) = version else {
    return bank.default_url();
  };
  match load_config(false).await {
    Ok(cfg) => cfg
      .version(version)
      .map_or_else(|| bank.default_url(), |v| v.resolve_url(bank)),
    Err(_) => bank.default_url(),
  }
}

async fn load_url(url: &str) -> Result<Questions, AppError> {
  if let Some(hit) = CACHE.with_borrow(|c| c.questions.get(url).cloned()) {
    return Ok(hit);
  }
  let text = fetch_text(url, CacheMode::ForceCache)
    .await
    .map_err(|e| AppError::Network(format!("Failed to load questions JSON: {e}")))?;
  let data: Vec<QuestionItem> = serde_json::from_str(&text)
    .map_err(|e| AppError::Validation(format!("Invalid questions JSON: {e}")))?;
  let data = Arc::new(data);
  CACHE.with_borrow_mut(|c| c.questions.insert(url.to_owned(), data.clone()));
  Ok(data)
}

/// 加载某版本的题库；`strict` 时空题库视为错误。
pub async fn load_bank(
  version: Option<&str>,
  bank: Bank,
  strict: bool,
) -> Result<Questions, AppError> {
  let url = resolve_url(version, bank).await;
  let rev = load_config(false)
    .await
    .ok()
    .and_then(|cfg| cfg.rev_of(&url).map(str::to_owned));
  let fetch_url = rev
    .as_ref()
    .map_or_else(|| url.clone(), |r| format!("{url}?v={r}"));
  let fresh = CACHE.with_borrow(|c| !c.questions.contains_key(&fetch_url));
  let data = load_url(&fetch_url).await?;
  if fresh && url == bank.default_url() {
    crate::bank_updates::observe(bank, rev.as_deref().unwrap_or_default(), &data);
  }
  if strict && data.is_empty() {
    return Err(AppError::Validation(format!(
      "Questions for bank {bank} (version {}) empty",
      version.unwrap_or("default")
    )));
  }
  Ok(data)
}

/// 题库是否可用（存在且非空）。
pub async fn bank_available(version: Option<&str>, bank: Bank) -> bool {
  load_bank(version, bank, true).await.is_ok()
}

async fn check_version(version_id: &str) -> VersionStatus {
  let mut available_banks = Vec::new();
  for bank in Bank::ALL {
    if bank_available(Some(version_id), bank).await {
      available_banks.push(bank);
    }
  }
  VersionStatus {
    version_id: version_id.to_owned(),
    is_available: !available_banks.is_empty(),
    available_banks,
    last_checked: now_ms(),
    error: None,
  }
}

/// 获取版本状态（可用 1 分钟、不可用 10 秒缓存）。
pub async fn version_status(version_id: &str, force: bool) -> VersionStatus {
  if !force {
    let cached = CACHE.with_borrow(|c| c.status.get(version_id).cloned());
    if let Some(s) = cached {
      let max_age = if s.is_available {
        CACHE_DURATION
      } else {
        SHORT_CACHE_DURATION
      };
      if now_ms() - s.last_checked < max_age {
        return s;
      }
    }
  }
  if force {
    CACHE.with_borrow_mut(|c| c.questions.clear());
  }
  let status = check_version(version_id).await;
  CACHE.with_borrow_mut(|c| c.status.insert(version_id.to_owned(), status.clone()));
  status
}

/// 加载失败后的最短重试间隔（毫秒）。
///
/// 失败结果**不会**写入常驻缓存（否则一次网络抖动会让整个会话都拿不到数据），
/// 但也不能因此每帧都重试 —— 离线时那样会打出一串必然失败的请求。
const RETRY_BACKOFF_MS: i64 = 30_000;

// 上次加载失败的时间戳（毫秒），0 表示从未失败。
thread_local! {
  static GLOSSARY_FAILED_AT: Cell<i64> = const { Cell::new(0) };
  static INDEX_FAILED_AT: Cell<i64> = const { Cell::new(0) };
}

/// 距上次失败是否已超过退避窗口（可以再试一次）。
fn backoff_elapsed(failed_at: &'static std::thread::LocalKey<Cell<i64>>) -> bool {
  let last = failed_at.with(Cell::get);
  if last == 0 {
    return true;
  }
  // 系统时钟被回拨时 `now < last`，差值会为负；此时按「已过期」处理，否则退避窗口
  // 在下一次时钟追上之前会一直不生效。
  let now = now_ms();
  now < last || now - last >= RETRY_BACKOFF_MS
}

/// 记录一次失败时间戳。
fn mark_failed(failed_at: &'static std::thread::LocalKey<Cell<i64>>) {
  failed_at.with(|c| c.set(now_ms()));
}

static GLOSSARY: OnceLock<Glossary> = OnceLock::new();
/// 加载失败时返回的空术语表。
///
/// 单独一个 `OnceLock`：它只作为「本次失败」的返回值，不污染 [`GLOSSARY`]，
/// 因此退避窗口过后仍能重新加载。
static EMPTY_GLOSSARY: OnceLock<Glossary> = OnceLock::new();

/// 已加载的术语表（尚未加载时为 `None`）。
pub fn glossary_loaded() -> Option<&'static Glossary> {
  GLOSSARY.get()
}

/// 术语表：首次使用时拉取 `/data/glossary/*.json`（按分类拆分，Service Worker 预缓存，离线可用），
/// 之后复用内存缓存。不嵌入 wasm 以减小首屏下载体积。
pub async fn load_glossary() -> &'static Glossary {
  if let Some(g) = GLOSSARY.get() {
    return g;
  }
  if !backoff_elapsed(&GLOSSARY_FAILED_AT) {
    return EMPTY_GLOSSARY.get_or_init(Glossary::default);
  }
  let mut parsed = Vec::with_capacity(GLOSSARY_FILES.len());
  for name in GLOSSARY_FILES {
    let url = format!("/data/glossary/{name}.json");
    match fetch_text(&url, CacheMode::Default).await {
      Ok(text) => match serde_json::from_str::<Glossary>(&text) {
        Ok(g) => parsed.push(g),
        Err(e) => web_sys::console::error_1(
          &tf(
            "[ERROR] 术语表 {} 解析失败：{}",
            &[(name), &(e).to_string()],
          )
          .into(),
        ),
      },
      Err(e) => web_sys::console::error_1(
        &tf(
          "[ERROR] 术语表 {} 加载失败：{}",
          &[(name), &(e).to_string()],
        )
        .into(),
      ),
    }
  }
  if parsed.len() < GLOSSARY_FILES.len() {
    // 有任一文件解析失败就不写入常驻缓存（否则一次网络抖动会让整个会话拿到不完整的
    // 术语表），记为失败、退避后再试。
    mark_failed(&GLOSSARY_FAILED_AT);
    return EMPTY_GLOSSARY.get_or_init(Glossary::default);
  }
  GLOSSARY_FAILED_AT.with(|c| c.set(0));
  GLOSSARY.get_or_init(|| Glossary::merged(parsed))
}

thread_local! {
  /// 当前译文：`(语言, 中文原文 → 译文)`，按模块分批翻译、构建时合并导出。
  ///
  /// 连语言一起存，是因为切换界面语言时必须能看出「手里这份是哪一语言的」——
  /// 只存词典的话，zh → en 之后切到 es 会被「已加载」短路掉，西班牙语界面显示英文译文。
  ///
  /// 用 `RefCell` 而非 `OnceLock`：切语言要整体替换，而 `OnceLock` 只在首次写入生效。
  /// 所有读写都在主线程（wasm 单线程），不存在并发竞争。
  static KNOWLEDGE_I18N: RefCell<Option<(String, HashMap<String, String>)>> =
    const { RefCell::new(None) };
  /// 译文加载完成的版本号。
  ///
  /// 词典是异步拉取的，而知识页的条目是 `&'static` 切片、只会构建一次视图 —— 不通知
  /// 响应式系统的话，译文到达后页面不会重算，仍然停在中文。组件通过 [`track_knowledge`]
  /// 订阅这个信号。
  ///
  /// 用 `ArcRwSignal` 而非 `RwSignal`：`RwSignal` 是 arena 分配的，会随创建时所在的
  /// 响应式 `Owner` 一起被释放，而首次订阅发生在页面组件内部 —— 离开该页面后这个
  /// 「全局」信号就失效了。引用计数信号只要还有引用就一直有效。
  static KNOWLEDGE_READY: ArcRwSignal<u32> = ArcRwSignal::new(0);
  /// 译文加载失败的时间戳（毫秒），0 表示从未失败（同 [`GLOSSARY_FAILED_AT`]）。
  static KNOWLEDGE_FAILED_AT: Cell<i64> = const { Cell::new(0) };
}

/// 订阅知识库译文的加载状态；译文到达后调用方会重新求值。
pub fn track_knowledge() {
  KNOWLEDGE_READY.with(|s| s.track());
}

/// 当前译文是否已就绪且属于 `lang`。
///
/// 调用方应拿它判断是否需要加载，而不是只看「有没有词典」—— 后者在切语言后会误判为就绪。
#[must_use]
pub fn knowledge_i18n_ready(lang: &str) -> bool {
  KNOWLEDGE_I18N.with(|c| c.borrow().as_ref().is_some_and(|(l, _)| l == lang))
}

/// 加载某语言的知识库正文译文。
///
/// 词典可能很大（全部模块合计可达数万条），因此只在切到非中文界面时才拉取；
/// 拉取失败不缓存空结果，退避后可重试（同 [`load_glossary`]）。
///
/// 已缓存的语言与 `lang` 不一致时会重新拉取 —— 否则 en → es 会一直沿用英文词典。
pub async fn load_knowledge_i18n(lang: &str) {
  if knowledge_i18n_ready(lang) {
    return;
  }
  if !backoff_elapsed(&KNOWLEDGE_FAILED_AT) {
    return;
  }
  if lang == "zh" {
    // 中文无需译文，空词典即可（同样记下语言，切走再切回来才不会误判）。
    set_knowledge_i18n(lang.to_owned(), HashMap::new());
    KNOWLEDGE_FAILED_AT.with(|c| c.set(0));
    return;
  }
  let url = format!("/data/knowledge-i18n/{lang}.json");
  let dict: Option<HashMap<String, String>> = match fetch_text(&url, CacheMode::Default).await {
    Ok(text) => serde_json::from_str(&text).ok(),
    Err(_) => None,
  };
  // 空词典不落常驻缓存：否则一次拉取失败（或尚未翻译）就会让后续请求一直走不到重试。
  if let Some(d) = dict.filter(|d| !d.is_empty()) {
    set_knowledge_i18n(lang.to_owned(), d);
    KNOWLEDGE_FAILED_AT.with(|c| c.set(0));
  } else {
    mark_failed(&KNOWLEDGE_FAILED_AT);
  }
}

/// 替换当前译文并通知订阅方重算。
fn set_knowledge_i18n(lang: String, dict: HashMap<String, String>) {
  KNOWLEDGE_I18N.with(|c| *c.borrow_mut() = Some((lang, dict)));
  KNOWLEDGE_READY.with(|s| s.update(|n| *n += 1));
}

/// 翻译一条知识库正文；无译文时回退中文原文。
///
/// 与界面文案的 `t()` 分开，是因为两者的 key 来源不同：界面文案以中文原文为 key 写在
/// `i18n.rs` 里，知识库正文则由 `data/knowledge-i18n/` 按模块维护。
#[must_use]
pub fn kt(text: &str) -> String {
  KNOWLEDGE_I18N.with(|c| {
    c.borrow()
      .as_ref()
      .and_then(|(_, d)| d.get(text).cloned())
      .unwrap_or_else(|| text.to_owned())
  })
}

static QUESTION_INDEX: OnceLock<Vec<QuestionSearchEntry>> = OnceLock::new();
/// 加载失败时返回的空索引（同 [`EMPTY_GLOSSARY`]，不污染常驻缓存）。
static EMPTY_INDEX: OnceLock<Vec<QuestionSearchEntry>> = OnceLock::new();

/// 已加载的题目搜索索引（尚未加载时为 `None`）。
pub fn question_index_loaded() -> Option<&'static Vec<QuestionSearchEntry>> {
  QUESTION_INDEX.get()
}

/// 题目搜索索引：首次使用时拉取 `/questions/search-index.json`（由 `postbuild` 生成，
/// 仅含题干 + 解析，不增大首屏），之后复用内存缓存。
pub async fn load_question_index() -> &'static Vec<QuestionSearchEntry> {
  if let Some(idx) = QUESTION_INDEX.get() {
    return idx;
  }
  if !backoff_elapsed(&INDEX_FAILED_AT) {
    return EMPTY_INDEX.get_or_init(Vec::new);
  }
  let entries = match fetch_text("/questions/search-index.json", CacheMode::Default).await {
    Ok(text) => serde_json::from_str::<Vec<QuestionSearchEntry>>(&text).unwrap_or_default(),
    Err(_) => Vec::new(),
  };
  if entries.is_empty() {
    mark_failed(&INDEX_FAILED_AT);
    return EMPTY_INDEX.get_or_init(Vec::new);
  }
  INDEX_FAILED_AT.with(|c| c.set(0));
  QUESTION_INDEX.get_or_init(|| entries)
}

/// 强制刷新配置与全部版本状态。
pub async fn refresh_all() -> Result<(), AppError> {
  let versions = all_versions(true).await?;
  for v in &versions {
    version_status(&v.id, true).await;
  }
  Ok(())
}
