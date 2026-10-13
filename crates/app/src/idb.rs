//! IndexedDB 轻量 KV 封装：承载大体积数据（通联日志），突破 localStorage 5MB 配额。
//!
//! 单 object store（`kv`）的 key-value 模式；异步 API 基于 `wasm-bindgen-futures` 把
//! 回调式 `Idb*` 请求包装为 `Future`（与 [`crate::util::read_file_text`] 同一套路）。
//!
//! 数据库连接缓存于线程局部，避免重复 open。此外内置一层**进程内内存缓存**：写入时
//! 同步更新缓存、读取时先命中缓存，既减少 IndexedDB 往返，也保证「写后立即可读」且
//! 不会被尚未返回的异步读取用旧值覆盖。跨标签页的可见性由上层（[`crate::kv`] 的
//! localStorage 快照 + `storage` 事件）负责，本层不处理。

use std::cell::RefCell;
use std::collections::HashMap;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbRequest, IdbTransactionMode};

const DB_NAME: &str = "ham-web";
const STORE: &str = "kv";
const VERSION: u32 = 1;

/// 连接事件闭包（`close` / `versionchange`）：必须活得和连接一样久，换连接时一并释放。
type DbHandlers = (
  wasm_bindgen::closure::Closure<dyn FnMut()>,
  wasm_bindgen::closure::Closure<dyn FnMut()>,
);

thread_local! {
  static DB: RefCell<Option<IdbDatabase>> = const { RefCell::new(None) };
  static DB_HANDLERS: RefCell<Option<DbHandlers>> = const { RefCell::new(None) };
  /// 进程内缓存：`get` 的快速路径；`set`/`remove` 同步维护，保证读写一致。
  static CACHE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
  static REVISIONS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

/// 读内存缓存；未加载过返回 `None`。
pub(crate) fn cache_get(key: &str) -> Option<String> {
  CACHE.with(|c| c.borrow().get(key).cloned())
}

fn revision(key: &str) -> u64 {
  REVISIONS.with(|versions| versions.borrow().get(key).copied().unwrap_or(0))
}

fn bump_revision(key: &str) -> u64 {
  REVISIONS.with(|versions| {
    let mut versions = versions.borrow_mut();
    let next = versions.get(key).copied().unwrap_or(0).wrapping_add(1);
    versions.insert(key.to_owned(), next);
    next
  })
}

fn commit_cache_if_current(key: &str, token: u64, value: Option<&str>) -> bool {
  if revision(key) != token {
    return false;
  }
  CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    if let Some(value) = value {
      cache.insert(key.to_owned(), value.to_owned());
    } else {
      cache.remove(key);
    }
  });
  true
}

/// 写内存缓存（同步，供落盘前的即时读取）。
///
/// `pub(crate)`：供 [`crate::kv::save_large`] 在 spawn 前同步写入缓存，保证
/// 写后立即 [`get`] 可读，而非等异步任务被调度后才生效。
pub(crate) fn cache_put(key: &str, value: &str) {
  bump_revision(key);
  CACHE.with(|c| {
    c.borrow_mut().insert(key.to_owned(), value.to_owned());
  });
}

/// 移除内存缓存。
pub(crate) fn cache_remove(key: &str) {
  bump_revision(key);
  CACHE.with(|c| {
    c.borrow_mut().remove(key);
  });
}

/// 把 `IdbRequest` 的 onsuccess / onerror 包装为 `Future`，resolve 请求的 `result`。
///
/// 两个处理闭包只会触发其一：用 `once_into_js` 的话没触发的那个永久泄漏。这里让它们活到
/// future 结束再一起释放（并摘掉事件处理函数）。失败时带上真实的 `DOMException`
/// （`QuotaExceededError` 等），不再丢成一句笼统的字符串。
async fn request_future(request: &IdbRequest) -> Result<JsValue, JsValue> {
  type Handler = wasm_bindgen::closure::Closure<dyn FnMut()>;
  let mut handlers: Option<(Handler, Handler)> = None;
  let settled = JsFuture::from(Promise::new(&mut |resolve, reject| {
    let ok_req = request.clone();
    let ok = Handler::new(move || {
      let result = ok_req.result().unwrap_or(JsValue::UNDEFINED);
      let _ = resolve.call1(&JsValue::NULL, &result);
    });
    let err_req = request.clone();
    let err = Handler::new(move || {
      // `IdbRequest.error` 是 DOMException；经 Reflect 读取，免得为它多开 web-sys 特性。
      let reason = js_sys::Reflect::get(&err_req, &"error".into())
        .ok()
        .filter(|e| !e.is_null() && !e.is_undefined())
        .unwrap_or_else(|| JsValue::from_str("indexedDB request failed"));
      let _ = reject.call1(&JsValue::NULL, &reason);
    });
    request.set_onsuccess(Some(ok.as_ref().unchecked_ref()));
    request.set_onerror(Some(err.as_ref().unchecked_ref()));
    handlers = Some((ok, err));
  }));
  let outcome = settled.await;
  request.set_onsuccess(None);
  request.set_onerror(None);
  drop(handlers);
  outcome
}

/// 连接断开（浏览器回收 / 清站点数据）或被别的标签页升级时，丢弃缓存的连接：
/// 否则 `transaction()` 会一直抛 `InvalidStateError`，直到刷新页面。
fn forget_db() {
  DB.with(|c| *c.borrow_mut() = None);
  DB_HANDLERS.with(|h| *h.borrow_mut() = None);
}

/// 打开（或复用）数据库连接；首次会创建 `kv` object store。
async fn open_db() -> Result<IdbDatabase, JsValue> {
  if let Some(db) = DB.with(|c| c.borrow().clone()) {
    return Ok(db);
  }
  let factory = crate::util::window()
    .indexed_db()?
    .ok_or_else(|| JsValue::from_str("indexedDB unavailable"))?;
  let request = factory.open_with_u32(DB_NAME, VERSION)?;

  let upgrade =
    wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
      let req = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::IdbOpenDbRequest>().ok());
      let Some(db) = req
        .and_then(|r| r.result().ok())
        .map(|v| v.unchecked_into::<IdbDatabase>())
      else {
        return;
      };
      // onupgradeneeded 仅在数据库版本变化（首次创建 / 升级）时触发；已存在同名 store
      // 时 create_object_store 会抛错，静默忽略即可。
      let _ = db.create_object_store(STORE);
    });
  request.set_onupgradeneeded(Some(upgrade.as_ref().unchecked_ref()));

  let result = request_future(&request).await;
  request.set_onupgradeneeded(None);
  drop(upgrade);
  let db: IdbDatabase = result?.unchecked_into();

  // 并发首次调用：别人已经存好了连接，就关掉自己这条，避免两条连接互相覆盖、前者不关。
  if let Some(existing) = DB.with(|c| c.borrow().clone()) {
    db.close();
    return Ok(existing);
  }
  let on_close = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(forget_db);
  let on_version_change = wasm_bindgen::closure::Closure::<dyn FnMut()>::new({
    let db = db.clone();
    move || {
      // 别的标签页要升级库：必须主动关闭，否则对方的升级被无限期阻塞。
      db.close();
      forget_db();
    }
  });
  db.set_onclose(Some(on_close.as_ref().unchecked_ref()));
  db.set_onversionchange(Some(on_version_change.as_ref().unchecked_ref()));
  DB_HANDLERS.with(|h| *h.borrow_mut() = Some((on_close, on_version_change)));
  DB.with(|c| *c.borrow_mut() = Some(db.clone()));
  Ok(db)
}

/// 读取键值；键不存在返回 `Ok(None)`。优先命中内存缓存，未命中再查 IndexedDB 并回填。
pub async fn get(key: &str) -> Result<Option<String>, JsValue> {
  if let Some(v) = cache_get(key) {
    return Ok(Some(v));
  }
  let token = revision(key);
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(STORE)?;
  let req = store.get(&JsValue::from_str(key))?;
  let result = request_future(&req).await?;
  if revision(key) != token {
    return Ok(cache_get(key));
  }
  let value = result.as_string();
  if let Some(v) = &value {
    commit_cache_if_current(key, token, Some(v));
  }
  Ok(value)
}

/// 读取键值但**不回填**进程内缓存：给 QSL 扫描件这类体积大、无同步读需求的数据。
///
/// 回填的话每张看过 / 导出过的影像（数百 KB 到数 MB 的 data URL）都会常驻内存、永不淘汰，
/// 备份导出一次读完所有影像可累计到 64 MB 量级。
pub async fn get_uncached(key: &str) -> Result<Option<String>, JsValue> {
  if let Some(v) = cache_get(key) {
    return Ok(Some(v));
  }
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readonly)?;
  let req = tx.object_store(STORE)?.get(&JsValue::from_str(key))?;
  Ok(request_future(&req).await?.as_string())
}

/// 写入键值（字符串）：落盘 IndexedDB。
///
/// 前置条件：调用方必须先用 [`cache_put`] 把**同一个值**写进内存缓存
/// （[`crate::kv::save_large`] 就是这么做的）。这个「缓存必须恰好是本值」的检查同时
/// 充当并发判据：缓存被更新的值顶掉、或已被删除时直接跳过，避免旧提交盖住新值或
/// 复活已删数据；不先 prime 缓存就调用会静默不落盘。
pub async fn set(key: &str, value: &str) -> Result<(), JsValue> {
  if cache_get(key).as_deref() != Some(value) {
    return Ok(());
  }
  let token = revision(key);
  let db = open_db().await?;
  if revision(key) != token {
    return Ok(());
  }
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  store.put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
  // 等事务 `complete` 而不是请求 `onsuccess`：配额错误在提交阶段才以 abort 出现，只等请求会
  // 把写失败当成功（`kv::save_large` 的 console.error 因此抓不到）。
  transaction_complete(&tx).await
}

/// 备份写入等待事务提交，只有请求成功不代表已经持久化。
pub async fn set_committed(key: &str, value: &str) -> Result<bool, JsValue> {
  let token = bump_revision(key);
  let db = open_db().await?;
  if revision(key) != token {
    return Ok(false);
  }
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  store.put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
  transaction_complete(&tx).await?;
  Ok(commit_cache_if_current(key, token, Some(value)))
}

/// 同 [`set_committed`]，但不写进程内缓存（见 [`get_uncached`]）；只保证旧缓存失效。
pub async fn set_committed_uncached(key: &str, value: &str) -> Result<bool, JsValue> {
  let token = bump_revision(key);
  // 先清掉可能存在的旧缓存，避免之后 `get_uncached` 命中过期值。
  CACHE.with(|c| c.borrow_mut().remove(key));
  let db = open_db().await?;
  if revision(key) != token {
    return Ok(false);
  }
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  tx.object_store(STORE)?
    .put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
  transaction_complete(&tx).await?;
  Ok(revision(key) == token)
}

/// 删除也等事务提交；旧上传提交后不能复活被删除的缓存。
pub async fn remove_committed(key: &str) -> Result<bool, JsValue> {
  let token = bump_revision(key);
  let db = open_db().await?;
  if revision(key) != token {
    return Ok(false);
  }
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  tx.object_store(STORE)?.delete(&JsValue::from_str(key))?;
  transaction_complete(&tx).await?;
  Ok(commit_cache_if_current(key, token, None))
}

async fn transaction_complete(tx: &web_sys::IdbTransaction) -> Result<(), JsValue> {
  let mut complete = None;
  let mut abort = None;
  let done = JsFuture::from(Promise::new(&mut |resolve, reject| {
    complete = Some(wasm_bindgen::closure::Closure::<dyn FnMut()>::new(
      move || {
        let _ = resolve.call0(&JsValue::NULL);
      },
    ));
    let tx_for_abort = tx.clone();
    abort = Some(wasm_bindgen::closure::Closure::<dyn FnMut()>::new(
      move || {
        // 透传事务的真实错误（`QuotaExceededError` 等）：笼统的「transaction aborted」
        // 会把配额失败这种真实原因盖掉，与 `request_future` 保留 DOMException 的口径一致。
        let err = js_sys::Reflect::get(&tx_for_abort, &"error".into())
          .ok()
          .filter(|v| !v.is_null() && !v.is_undefined())
          .unwrap_or_else(|| JsValue::from_str("indexedDB transaction aborted"));
        let _ = reject.call1(&JsValue::NULL, &err);
      },
    ));
    tx.set_oncomplete(complete.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
    tx.set_onabort(abort.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
  }));
  let outcome = done.await;
  tx.set_oncomplete(None);
  tx.set_onabort(None);
  drop((complete, abort));
  outcome.map(|_| ())
}

#[cfg(test)]
mod tests {
  use super::{bump_revision, cache_get, cache_put, cache_remove, commit_cache_if_current};

  /// 缓存层核心不变量：`set` 同步写入缓存后，`get` 必须先命中缓存返回最新值，
  /// 即使 IndexedDB 异步落盘尚未完成也不会用旧值覆盖（此处用纯缓存函数直接验证）。
  #[test]
  fn old_transaction_continuations_cannot_overwrite_new_writes_or_deletes() {
    let key = "__test_commit_revision";
    cache_put(key, "initial");
    let old = bump_revision(key);
    cache_put(key, "newer");
    assert!(!commit_cache_if_current(key, old, Some("old commit")));
    assert_eq!(cache_get(key).as_deref(), Some("newer"));
    let uploading = bump_revision(key);
    cache_remove(key);
    assert!(!commit_cache_if_current(key, uploading, Some("revived")));
    assert_eq!(cache_get(key), None);
    let latest = bump_revision(key);
    assert!(commit_cache_if_current(key, latest, Some("saved")));
    assert_eq!(cache_get(key).as_deref(), Some("saved"));
    cache_remove(key);
  }

  #[test]
  fn cache_write_then_read_returns_latest() {
    cache_put("__test_logbook", "v1");
    // 模拟「旧值尚未返回」：再次写入后读到的必须是新值。
    cache_put("__test_logbook", "v2");
    assert_eq!(cache_get("__test_logbook").as_deref(), Some("v2"));
    cache_remove("__test_logbook");
  }

  /// 未写入的 key 读不到，删除后也读不到。
  #[test]
  fn cache_miss_and_remove() {
    assert_eq!(cache_get("__test_missing"), None);
    cache_put("__test_missing", "1");
    cache_remove("__test_missing");
    assert_eq!(cache_get("__test_missing"), None);
  }

  /// 不同 key 之间互不串扰。
  #[test]
  fn cache_isolated_by_key() {
    cache_put("__test_a", "1");
    cache_put("__test_b", "2");
    assert_eq!(cache_get("__test_a").as_deref(), Some("1"));
    assert_eq!(cache_get("__test_b").as_deref(), Some("2"));
    cache_remove("__test_a");
    assert_eq!(cache_get("__test_a"), None);
    assert_eq!(cache_get("__test_b").as_deref(), Some("2"));
    cache_remove("__test_b");
  }
}
