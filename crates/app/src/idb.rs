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

thread_local! {
  static DB: RefCell<Option<IdbDatabase>> = const { RefCell::new(None) };
  /// 进程内缓存：`get` 的快速路径；`set`/`remove` 同步维护，保证读写一致。
  static CACHE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// 读内存缓存；未加载过返回 `None`。
fn cache_get(key: &str) -> Option<String> {
  CACHE.with(|c| c.borrow().get(key).cloned())
}

/// 写内存缓存（同步，供落盘前的即时读取）。
///
/// `pub(crate)`：供 [`crate::kv::save_large`] 在 spawn 前同步写入缓存，保证
/// 写后立即 [`get`] 可读，而非等异步任务被调度后才生效。
pub(crate) fn cache_put(key: &str, value: &str) {
  CACHE.with(|c| {
    c.borrow_mut().insert(key.to_owned(), value.to_owned());
  });
}

/// 移除内存缓存。
pub(crate) fn cache_remove(key: &str) {
  CACHE.with(|c| {
    c.borrow_mut().remove(key);
  });
}

/// 把 `IdbRequest` 的 onsuccess / onerror 包装为 `Future`，resolve 请求的 `result`。
fn request_future(request: &IdbRequest) -> JsFuture {
  let request = request.clone();
  JsFuture::from(Promise::new(&mut |resolve, reject| {
    let ok_req = request.clone();
    let ok = wasm_bindgen::closure::Closure::once_into_js(move || {
      let result = ok_req.result().unwrap_or(JsValue::UNDEFINED);
      let _ = resolve.call1(&JsValue::NULL, &result);
    });
    let err = wasm_bindgen::closure::Closure::once_into_js(move || {
      let _ = reject.call1(
        &JsValue::NULL,
        &JsValue::from_str("indexedDB request failed"),
      );
    });
    request.set_onsuccess(Some(ok.unchecked_ref()));
    request.set_onerror(Some(err.unchecked_ref()));
  }))
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

  let upgrade = wasm_bindgen::closure::Closure::once_into_js(move |ev: web_sys::Event| {
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
  request.set_onupgradeneeded(Some(upgrade.unchecked_ref()));

  let result = request_future(&request).await?;
  let db: IdbDatabase = result.unchecked_into();
  DB.with(|c| *c.borrow_mut() = Some(db.clone()));
  Ok(db)
}

/// 读取键值；键不存在返回 `Ok(None)`。优先命中内存缓存，未命中再查 IndexedDB 并回填。
pub async fn get(key: &str) -> Result<Option<String>, JsValue> {
  if let Some(v) = cache_get(key) {
    return Ok(Some(v));
  }
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readonly)?;
  let store = tx.object_store(STORE)?;
  let req = store.get(&JsValue::from_str(key))?;
  let result = request_future(&req).await?;
  let value = result.as_string();
  if let Some(v) = &value {
    cache_put(key, v);
  }
  Ok(value)
}

/// 写入键值（字符串）：先同步更新内存缓存，再落盘 IndexedDB。
pub async fn set(key: &str, value: &str) -> Result<(), JsValue> {
  cache_put(key, value);
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  let req = store.put_with_key(&JsValue::from_str(value), &JsValue::from_str(key))?;
  request_future(&req).await?;
  Ok(())
}

/// 删除键：先同步移除内存缓存，再删除 IndexedDB。
///
/// 当前业务尚无删除大数据的场景，保留作为 KV 封装的完整 API（经 [`crate::kv::remove_large`] 暴露）。
#[allow(dead_code)]
pub async fn remove(key: &str) -> Result<(), JsValue> {
  cache_remove(key);
  let db = open_db().await?;
  let tx = db.transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)?;
  let store = tx.object_store(STORE)?;
  let req = store.delete(&JsValue::from_str(key))?;
  request_future(&req).await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::{cache_get, cache_put, cache_remove};

  /// 缓存层核心不变量：`set` 同步写入缓存后，`get` 必须先命中缓存返回最新值，
  /// 即使 IndexedDB 异步落盘尚未完成也不会用旧值覆盖（此处用纯缓存函数直接验证）。
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
