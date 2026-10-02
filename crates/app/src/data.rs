//! 题库数据加载：配置文件、题库 JSON、版本可用性状态（带内存缓存）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use ham_web_core::glossary::{GLOSSARY_FILES, Glossary};
use ham_web_core::{Bank, BankConfig, QuestionItem, QuestionSearchEntry, QuestionVersion};
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

/// 拉取外部 JSON（带 5 秒超时，超时或失败返回错误）。
pub async fn fetch_external_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, AppError> {
  let text = fetch_text_external(url, 5000).await?;
  serde_json::from_str(&text).map_err(|e| AppError::Validation(format!("Invalid JSON: {e}")))
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

static GLOSSARY: OnceLock<Glossary> = OnceLock::new();

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
  GLOSSARY.get_or_init(|| Glossary::merged(parsed))
}

static QUESTION_INDEX: OnceLock<Vec<QuestionSearchEntry>> = OnceLock::new();

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
  let entries = match fetch_text("/questions/search-index.json", CacheMode::Default).await {
    Ok(text) => serde_json::from_str::<Vec<QuestionSearchEntry>>(&text).unwrap_or_default(),
    Err(_) => Vec::new(),
  };
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
